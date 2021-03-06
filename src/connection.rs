use crate::error::*;
use crate::*;
use std::collections::VecDeque;
use std::ops::Add;

#[derive(Eq, PartialEq, Debug)]
pub enum ConState {
    NewCreated,
    NotAuthenticated,
    Ready,
    Disconnected,
}

pub struct DBusPreallocatedSend {}

// TODO protect with mutex?
pub struct DBusPendingCall<'a> {
    serial: u32,
    ref_count: u64,
    timeout: Option<std::time::Instant>,
    reply: Option<*mut DBusMessage<'a>>,
    mutex: std::sync::Mutex<()>,
    cond: std::sync::Condvar,
}

impl<'a> DBusPendingCall<'a> {
    pub fn new(serial: u32, timeout: Option<std::time::Duration>) -> Self {
        DBusPendingCall {
            serial,
            ref_count: 1,
            reply: None,
            timeout: timeout.map(|timeout| std::time::Instant::now().add(timeout)),
            cond: std::sync::Condvar::new(),
            mutex: std::sync::Mutex::new(()),
        }
    }

    pub fn timed_out(&self) -> bool {
        if let Some(timeout) = self.timeout {
            timeout
                .checked_duration_since(std::time::Instant::now())
                .is_none()
        } else {
            false
        }
    }
}

#[repr(C)]
pub enum DBusDispatchStatus {
    Complete,
    DataRemaining,

    /// Unused. Rust has currently no easy way to detect OOM
    #[allow(unused)]
    NeedMemory,
}

pub struct MessageFilter {
    filter: DBusHandleMessageFunction,
    user_data: *mut std::ffi::c_void,
    free: crate::DBusFreeFunction,
}

#[repr(C)]
pub struct DBusConnection<'a> {
    pub con: rustbus::client_conn::Conn,
    pub ref_count: u64,
    pub state: ConState,
    pub exit_on_disconnect: bool,

    pub out_queue: VecDeque<*mut DBusMessage<'a>>,
    pub in_queue: VecDeque<*mut DBusMessage<'a>>,

    pub pending_calls: Vec<*mut DBusPendingCall<'a>>,

    pub unique_name: Option<std::ffi::CString>,

    pub route_peer_messages: bool,

    pub filters: Vec<MessageFilter>,
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
            pending_calls: Vec::new(),
            unique_name: None,
            route_peer_messages: false,
            filters: Vec::new(),
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

    pub fn dispatch_message(&mut self, mut msg: DBusMessage<'a>) {
        let self_ptr = self as *mut Self;

        if let rustbus::MessageType::Reply = msg.msg.typ {
            if let Some(reply_serial) = msg.msg.response_serial {
                for p in &mut self.pending_calls {
                    let p = unsafe { &mut **p };
                    if p.serial == reply_serial {
                        p.reply = Some(Box::into_raw(Box::new(msg)));
                        p.cond.notify_all();
                        return;
                    }
                }
            }
        }

        for filter in &self.filters {
            match (filter.filter)(self_ptr, &mut msg, filter.user_data) {
                DBusHandlerResult::DBUS_HANDLER_RESULT_HANDLED => {
                    return;
                }
                DBusHandlerResult::DBUS_HANDLER_RESULT_NEED_MEMORY => {
                    panic!("No OOM handling implemented");
                }
                DBusHandlerResult::DBUS_HANDLER_RESULT_NOT_YET_HANDLED => {
                    // Ok
                }
            }
        }

        // TODO implement handlers / fallbacks for object paths

        self.in_queue.push_back(Box::into_raw(Box::new(msg)))
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
pub extern "C" fn dbus_connection_open<'a>(
    addr: *const libc::c_char,
    err: *mut DBusError,
) -> *mut DBusConnection<'a> {
    let addr = unsafe {
        assert!(!addr.is_null());
        CStr::from_ptr(addr)
    };
    let path = addr.to_str().unwrap().to_string();

    match rustbus::client_conn::Conn::connect_to_bus(path.into(), false) {
        Ok(con) => Box::into_raw(Box::new(DBusConnection::new(con))),
        Err(e) => {
            if !err.is_null() {
                let err: &mut DBusError = unsafe { &mut *err };
                err.error = Box::new(format!("Could not connect to bus: {:?}", e));
            }
            std::ptr::null_mut()
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
            err.error = Box::new(format!("Unknown bus type: {:?}", bus));
            return std::ptr::null_mut();
        }
    };
    let con = match path {
        Ok(path) => match rustbus::client_conn::Conn::connect_to_bus(path, false) {
            Ok(mut con) => {
                con.send_message(&mut rustbus::standard_messages::hello(), None)
                    .unwrap();

                // TODO check this for an error response and return as such
                con.get_next_message(None).unwrap();
                Box::into_raw(Box::new(DBusConnection::new(con)))
            }
            Err(e) => {
                if !err.is_null() {
                    let err: &mut DBusError = unsafe { &mut *err };
                    err.error = Box::new(format!("Could not connect to bus: {:?}", e));
                }
                return std::ptr::null_mut();
            }
        },
        Err(e) => {
            if !err.is_null() {
                let err: &mut DBusError = unsafe { &mut *err };
                err.error = Box::new(format!("Could open path for bus: {:?}", e));
            }
            return std::ptr::null_mut();
        }
    };
    con
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
        return dbus_bool(false);
    }
    let con = unsafe { &mut *con };
    if msg.is_null() {
        return dbus_bool(false);
    }
    let msg = unsafe { &mut *msg };
    let new_serial = con.con.alloc_serial();
    msg.msg.serial = Some(new_serial);

    if !serial.is_null() {
        unsafe { *serial = new_serial };
    }

    // increase ref counter
    dbus_message_ref(msg);
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
pub extern "C" fn dbus_connection_read_write_dispatch(
    con: *mut DBusConnection,
    timeout: libc::c_int,
) -> u32 {
    dbus_connection_read_write(con, timeout);
    dbus_connection_dispatch(con);
    // TODO check for the Disconnect message and return false
    dbus_bool(true)
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
            Ok(msg) => con.dispatch_message(DBusMessage::new(msg)),
        }
    }
    if con.con.can_read_from_source().unwrap() {
        DBusDispatchStatus::DataRemaining
    } else {
        DBusDispatchStatus::Complete
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_send_with_reply<'a>(
    con: *mut DBusConnection<'a>,
    msg: *mut DBusMessage<'a>,
    pending: *mut *mut DBusPendingCall<'a>,
    timeout: libc::c_int,
) -> u32 {
    if con.is_null() {
        return dbus_bool(false);
    }
    let con = unsafe { &mut *con };
    if msg.is_null() {
        return dbus_bool(false);
    }
    let msg = unsafe { &mut *msg };
    if pending.is_null() {
        return dbus_bool(false);
    }
    let pending = unsafe { &mut *pending };

    let mut serial = 0u32;
    if dbus_connection_send(con, msg, &mut serial) == dbus_bool(false) {
        return dbus_bool(false);
    }

    let timeout = if timeout < 0 {
        // TODO what does libdbus use here as a 'sane' default (as they call it)?
        Some(std::time::Duration::from_millis(10))
    } else {
        Some(std::time::Duration::from_millis(timeout as u64))
    };
    let new_pending = Box::into_raw(Box::new(DBusPendingCall::new(serial, timeout)));
    *pending = new_pending;
    con.pending_calls.push(new_pending);
    dbus_bool(true)
}

#[no_mangle]
pub extern "C" fn dbus_connection_send_with_reply_and_block<'a>(
    con: *mut DBusConnection<'a>,
    msg: *mut DBusMessage<'a>,
    timeout: libc::c_int,
    err: *mut DBusError,
) -> *mut DBusMessage<'a> {
    if con.is_null() {
        return std::ptr::null_mut();
    }
    let con = unsafe { &mut *con };
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &mut *msg };
    let mut pending: *mut DBusPendingCall = std::ptr::null_mut();
    dbus_connection_send_with_reply(con, msg, &mut pending, timeout);

    // TODO convert error replys to DBusError
    let _ = err;

    let pending = unsafe { &mut *pending };
    while !pending.timed_out() {
        if let Some(reply) = pending.reply {
            return reply;
        }
        dbus_connection_read_write_dispatch(con, timeout);
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn dbus_connection_set_route_peer_messages<'a>(
    con: *mut DBusConnection<'a>,
    value: u32,
) {
    if con.is_null() {
        return;
    }
    let con = unsafe { &mut *con };
    con.route_peer_messages = value != 0;
}

#[no_mangle]
pub extern "C" fn dbus_connection_add_filter<'a>(
    con: *mut DBusConnection<'a>,
    filter: DBusHandleMessageFunction,
    user_data: *mut std::ffi::c_void,
    free: crate::DBusFreeFunction,
) -> u32 {
    if con.is_null() {
        return dbus_bool(false);
    }
    let con = unsafe { &mut *con };
    con.filters.push(MessageFilter {
        filter,
        user_data,
        free,
    });

    dbus_bool(true)
}

#[no_mangle]
pub extern "C" fn dbus_connection_remove_filter<'a>(
    con: *mut DBusConnection<'a>,
    filter: DBusHandleMessageFunction,
    _user_data: *mut std::ffi::c_void,
) {
    if con.is_null() {
        return;
    }
    let con = unsafe { &mut *con };

    // this is necessaey because the DBusHandleMessageFunctions cannot be compared directly
    let filter_ptr: *const std::ffi::c_void = unsafe { std::mem::transmute(filter) };
    for f in &con.filters {
        let f_ptr: *const std::ffi::c_void = unsafe { std::mem::transmute(f.filter) };
        if f_ptr == filter_ptr {}
    }

    // FIXME what happens with the userdata?
    // Does it have to be the same as the original userdata?
    // HUH?
}
