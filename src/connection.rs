use crate::*;
use std::collections::VecDeque;

#[derive(Eq, PartialEq, Debug)]
pub enum ConState {
    NewCreated,
    NotAuthenticated,
    Ready,
    Disconnected,
}

pub struct DBusPreallocatedSend {}

#[repr(C)]
pub enum DBusDispatchStatus {
    Complete,
    DataRemaining,
    NeedMemory,
}

pub struct DBusConnection<'a> {
    con: rustbus::client_conn::Conn,
    ref_count: u64,
    state: ConState,
    exit_on_disconnect: bool,

    out_queue: VecDeque<*mut DBusMessage<'a>>,
    in_queue: VecDeque<*mut DBusMessage<'a>>,
}

impl<'a> DBusConnection<'a> {
    pub fn new(con: rustbus::client_conn::Conn) -> Self {
        Self {
            con,
            ref_count: 1,
            state: ConState::Ready,
            exit_on_disconnect: false,
            out_queue: VecDeque::new(),
            in_queue: VecDeque::new(),
        }
    }

    pub fn send_next_message(&mut self, timeout: Option<std::time::Duration>) {
        if let Some(msg) = self.out_queue.pop_front() {
            if !msg.is_null() {
                let msg = unsafe { &mut *msg };
                self.con.send_message(&mut msg.msg, timeout).unwrap();
            }
        }
    }
}

impl<'a> Drop for DBusConnection<'a> {
    fn drop(&mut self) {
        for msg in &mut self.out_queue {
            crate::message::dbus_message_unref(*msg);
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
            Ok(con) => Box::into_raw(Box::new(DBusConnection::new(con))),
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
    match con
        .con
        .send_message(&mut rustbus::standard_messages::hello(), None)
    {
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
    let con = unsafe { &mut *con };
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };
    let new_serial = con.con.alloc_serial();
    msg.msg.serial = Some(new_serial);
    unsafe { *serial = new_serial };
    con.out_queue.push_back(msg);
    dbus_bool(true)
}

#[no_mangle]
pub extern "C" fn dbus_connection_ref(con: *mut DBusConnection) -> *mut DBusConnection {
    if con.is_null() {
        return con;
    }
    let con = unsafe { &mut *con };
    con.ref_count += 1;
    return con;
}

#[no_mangle]
pub extern "C" fn dbus_connection_unref(con: *mut DBusConnection) {
    if con.is_null() {
        return;
    }
    let con = unsafe { &mut *con };
    con.ref_count -= 1;
    if con.ref_count == 0 {
        dbus_connection_close(con);
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

#[no_mangle]
pub extern "C" fn dbus_connection_flush(con: *mut DBusConnection) {
    if con.is_null() {
        return;
    }
    let con = unsafe { &mut *con };
    while !con.out_queue.is_empty() {
        con.send_next_message(None);
    }
}

fn calc_remaining_time(
    start: &std::time::Instant,
    timeout: &Option<std::time::Duration>,
) -> Result<Option<std::time::Duration>, ()> {
    match timeout {
        None => Ok(None),
        Some(d) => {
            let elapsed = start.elapsed();
            if elapsed >= *d {
                Err(())
            } else {
                Ok(Some(*d - elapsed))
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_read_write(
    con: *mut DBusConnection,
    timeout: libc::c_int,
) -> u32 {
    if con.is_null() {
        return dbus_bool(false);
    }
    let con = unsafe { &mut *con };
    if con.state == ConState::Disconnected {
        return dbus_bool(false);
    }

    let timeout = if timeout < 0 {
        Some(std::time::Duration::from_millis(timeout as u64))
    } else {
        None
    };
    // TODO the doc is not exactly clear on the semantics here
    let start = std::time::Instant::now();
    while !con.out_queue.is_empty() {
        if let Ok(timeout) = calc_remaining_time(&start, &timeout) {
            con.send_next_message(timeout);
        }
    }

    match con.con.can_read_from_source() {
        Ok(true) => {
            if let Ok(timeout) = calc_remaining_time(&start, &timeout) {
                if let Err(_e) = con.con.read_once(timeout) {
                    // TODO more cleanup
                    con.state = ConState::Disconnected;
                    return dbus_bool(false);
                }
            }
        }
        Ok(false) => {
            // nothing to do
        }
        Err(_e) => {
            // TODO more cleanup
            con.state = ConState::Disconnected;
            return dbus_bool(false);
        }
    }

    return dbus_bool(true);
}

#[no_mangle]
pub extern "C" fn dbus_connection_dispatch(con: *mut DBusConnection) -> DBusDispatchStatus {
    if con.is_null() {
        return DBusDispatchStatus::Complete;
    }
    let con = unsafe { &mut *con };
    if con.con.buffer_contains_whole_message().unwrap() {
        match con
            .con
            .get_next_message(Some(std::time::Duration::from_micros(0)))
        {
            Err(_e) => {
                // TODO
            }
            Ok(msg) => con
                .in_queue
                .push_back(Box::into_raw(Box::new(DBusMessage::new(msg)))),
        }
    }
    if con.con.can_read_from_source().unwrap() {
        DBusDispatchStatus::DataRemaining
    } else {
        DBusDispatchStatus::Complete
    }
}
