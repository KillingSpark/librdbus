use crate::*;

pub enum ConState {
    NewCreated,
    NotAuthenticated,
    Ready,
    Disconnected,
}

pub struct DBusConnection<'a> {
    con: rustbus::client_conn::RpcConn<'a, 'a>,
    ref_count: u64,
    state: ConState,
    exit_on_disconnect: bool,
}

impl<'a> DBusConnection<'a> {
    pub fn new(con: rustbus::client_conn::RpcConn<'a, 'a>) -> Self {
        Self {
            con,
            ref_count: 1,
            state: ConState::Ready,
            exit_on_disconnect: false,
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_bus_get<'a>(
    bus: DBusBusType,
    err: *mut DBusError,
) -> *mut DBusConnection<'a> {
    let path = match bus {
        DBusBusType::DBUS_BUS_SESSION => rustbus::get_session_bus_path(),
        DBusBusType::DBUS_BUS_SYSTEM => rustbus::get_system_bus_path(),
        _ => {
            let err: &mut DBusError = unsafe { &mut *err };
            err.error = format!("Unknown bus type: {:?}", bus);
            return std::ptr::null_mut();
        }
    };
    match path {
        Ok(path) => match rustbus::client_conn::Conn::connect_to_bus(path, false) {
            Ok(con) => Box::into_raw(Box::new(DBusConnection::new(
                rustbus::client_conn::RpcConn::new(con),
            ))),
            Err(e) => {
                if !err.is_null() {
                    let err: &mut DBusError = unsafe { &mut *err };
                    err.error = format!("Could not connect to bus: {:?}", e);
                }
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            if !err.is_null() {
                let err: &mut DBusError = unsafe { &mut *err };
                err.error = format!("Could open path for bus: {:?}", e);
            }
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_send_hello<'a>(
    con: *mut DBusConnection<'a>,
    serial: *mut u32,
) -> u32 {
    if con.is_null() {
        return 0;
    }
    let con = unsafe { &mut *con };
    match con.con.send_message(&mut rustbus::standard_messages::hello(), None) {
        Ok(sent_serial) => {
            if !serial.is_null() {
                let serial = unsafe { &mut *serial };
                *serial = sent_serial;
            }
            1
        }
        Err(_e) => 0,
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_send<'a>(
    con: *mut DBusConnection<'a>,
    msg: *mut DBusMessage<'a>,
    serial: *mut u32,
) -> u32 {
    if con.is_null() {
        return 0;
    }
    if msg.is_null() {
        return 0;
    }
    let con = unsafe { &mut *con };
    let msg = unsafe { &mut *msg };
    let mut msg = msg.msg.clone();
    let r = con.con.send_message(&mut msg, None);

    match r {
        Ok(sent_serial) => {
            if !serial.is_null() {
                let serial = unsafe { &mut *serial };
                *serial = sent_serial;
            }
            1
        }
        Err(_e) => 0,
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_close(con: *mut DBusConnection) {
    if con.is_null() {
        return;
    }
    unsafe {
        Box::from_raw(con);
        //dropped here -> free'd
    }
}
