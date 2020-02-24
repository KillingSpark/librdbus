#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod message;
mod message_iter;
use message::*;
use std::ffi::CStr;

use libc;

#[repr(C)]
#[derive(Debug)]
pub enum DBusBusType {
    DBUS_BUS_SESSION,
    DBUS_BUS_SYSTEM,
    DBUS_BUS_STARTER,
}


pub const DBUS_TYPE_INVALID: libc::c_int = 0 as libc::c_int;
pub const DBUS_TYPE_STRING: libc::c_int = b's' as libc::c_int;
pub const DBUS_TYPE_BYTE: libc::c_int = b'y' as libc::c_int;
pub const DBUS_TYPE_BOOLEAN: libc::c_int = b'b' as libc::c_int;
pub const DBUS_TYPE_INT16: libc::c_int = b'n' as libc::c_int;
pub const DBUS_TYPE_UINT16: libc::c_int = b'q' as libc::c_int;
pub const DBUS_TYPE_INT32: libc::c_int = b'i' as libc::c_int;
pub const DBUS_TYPE_UINT32: libc::c_int = b'u' as libc::c_int;
pub const DBUS_TYPE_INT64: libc::c_int = b'x' as libc::c_int;
pub const DBUS_TYPE_UINT64: libc::c_int = b't' as libc::c_int;
pub const DBUS_TYPE_DOUBLE: libc::c_int = b'd' as libc::c_int;
pub const DBUS_TYPE_OBJECTPATH: libc::c_int = b'o' as libc::c_int;
pub const DBUS_TYPE_SIGNATURE: libc::c_int = b'g' as libc::c_int;
pub const DBUS_TYPE_UNIXFD: libc::c_int = b'h' as libc::c_int;
pub const DBUS_TYPE_ARRAY: libc::c_int = b'a' as libc::c_int;
pub const DBUS_TYPE_VARIANT: libc::c_int = b'v' as libc::c_int;
pub const DBUS_TYPE_STRUCT: libc::c_int = b'r' as libc::c_int;
pub const DBUS_TYPE_DICTENTRY: libc::c_int = b'e' as libc::c_int;

type DBusConnection = rustbus::client_conn::RpcConn;

#[repr(C)]
pub struct Error {
    is_set: bool,
    error: String,
}

#[no_mangle]
pub extern "C" fn dbus_error_init(err: *mut Error) {
    let err = unsafe { &mut *err };
    err.error = String::new();
    err.is_set = false;
}

#[no_mangle]
pub extern "C" fn dbus_error_is_set(err: *mut Error) -> libc::c_int {
    if err.is_null() {
        return 0;
    }

    let err: &mut Error = unsafe { &mut *err };
    if err.is_set {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_bus_get(
    bus: DBusBusType,
    err: *mut Error,
) -> *mut rustbus::client_conn::RpcConn {
    let path = match bus {
        DBusBusType::DBUS_BUS_SESSION => rustbus::get_session_bus_path(),
        DBusBusType::DBUS_BUS_SYSTEM => rustbus::get_system_bus_path(),
        _ => {
            let err: &mut Error = unsafe { &mut *err };
            err.error = format!("Unknown bus type: {:?}", bus);
            return std::ptr::null_mut();
        }
    };
    match path {
        Ok(path) => match rustbus::client_conn::Conn::connect_to_bus(path, false) {
            Ok(con) => Box::into_raw(Box::new(rustbus::client_conn::RpcConn::new(con))),
            Err(e) => {
                if !err.is_null() {
                    let err: &mut Error = unsafe { &mut *err };
                    err.error = format!("Could not connect to bus: {:?}", e);
                }
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            if !err.is_null() {
                let err: &mut Error = unsafe { &mut *err };
                err.error = format!("Could open path for bus: {:?}", e);
            }
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_send_hello(con: *mut DBusConnection, err: *mut Error) {
    if con.is_null() {
        return;
    }
    let con = unsafe { &mut *con };
    match con.send_message(rustbus::standard_messages::hello(), None) {
        Ok(_) => {}
        Err(e) => {
            if !err.is_null() {
                let err = unsafe { &mut *err };
                err.error = format!("Error sending message: {:?}", e);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_connection_send(
    con: *mut DBusConnection,
    msg: *mut DBusMessage,
    err: *mut Error,
) {
    if con.is_null() {
        return;
    }
    if msg.is_null() {
        return;
    }
    let con = unsafe { &mut *con };
    let msg = unsafe { &mut *msg };
    let msg = msg.msg.clone();
    let r = con.send_message(msg, None);
    

    if let Err(e) = r {
        if !err.is_null() {
            let err = unsafe { &mut *err };
            err.error = format!("Error sending message: {:?}", e);
        }
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

pub fn param_from_parts(
    argtyp: libc::c_int,
    arg: *mut std::ffi::c_void,
) -> Option<rustbus::message::Param> {
    let param: rustbus::message::Param = match argtyp {
        DBUS_TYPE_STRING => {
            let c_str = unsafe {
                assert!(!arg.is_null());
                let arg: *const *const libc::c_char = std::mem::transmute(arg);
                let arg = arg.read();
                assert!(!arg.is_null());
                CStr::from_ptr(arg)
            };
            let arg = c_str.to_str().unwrap().to_owned();
            arg.into()
        }
        DBUS_TYPE_OBJECTPATH => {
            let c_str = unsafe {
                assert!(!arg.is_null());
                let arg: *const *const libc::c_char = std::mem::transmute(arg);
                let arg = arg.read();
                assert!(!arg.is_null());
                CStr::from_ptr(arg)
            };
            let arg = c_str.to_str().unwrap().to_owned();
            rustbus::message::Base::ObjectPath(arg).into()
        }
        DBUS_TYPE_SIGNATURE => {
            let c_str = unsafe {
                assert!(!arg.is_null());
                let arg: *const *const libc::c_char = std::mem::transmute(arg);
                let arg = arg.read();
                assert!(!arg.is_null());
                CStr::from_ptr(arg)
            };
            let arg = c_str.to_str().unwrap().to_owned();
            rustbus::message::Base::ObjectPath(arg).into()
        }
        DBUS_TYPE_INT16 => {
            assert!(!arg.is_null());
            let ptr: *const i16 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Int16(val).into()
        }
        DBUS_TYPE_UINT16 => {
            assert!(!arg.is_null());
            let ptr: *const u16 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Uint16(val).into()
        }
        DBUS_TYPE_INT32 => {
            assert!(!arg.is_null());
            let ptr: *const i32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Int32(val).into()
        }
        DBUS_TYPE_UINT32 => {
            assert!(!arg.is_null());
            let ptr: *const u32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Uint32(val).into()
        }
        DBUS_TYPE_INT64 => {
            assert!(!arg.is_null());
            let ptr: *const i64 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Int64(val).into()
        }
        DBUS_TYPE_UINT64 => {
            assert!(!arg.is_null());
            let ptr: *const u64 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Uint64(val).into()
        }
        DBUS_TYPE_BOOLEAN => {
            assert!(!arg.is_null());
            let ptr: *const u32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Boolean(val != 0).into()
        }
        DBUS_TYPE_BYTE => {
            assert!(!arg.is_null());
            let ptr: *const u8 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Byte(val).into()
        }
        DBUS_TYPE_DOUBLE => {
            assert!(!arg.is_null());
            let ptr: *const u64 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::Double(val).into()
        }
        DBUS_TYPE_UNIXFD => {
            assert!(!arg.is_null());
            let ptr: *const u32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            rustbus::message::Base::UnixFd(val).into()
        }
        _ => return None,
    };
    Some(param)
}

pub fn write_base_param(param: &rustbus::message::Base, arg: *mut std::ffi::c_void) {
    match param {
        rustbus::message::Base::Boolean(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = if *val { 1 } else { 0 };
        }
        rustbus::message::Base::Byte(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u8 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Int16(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i16 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Uint16(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u16 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Int32(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i32 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Uint32(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Int64(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i64 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Uint64(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u64 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::Double(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u64 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::UnixFd(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        rustbus::message::Base::String(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            *mutref = unsafe { std::mem::transmute(val.as_ptr()) };
        }
        rustbus::message::Base::ObjectPath(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            *mutref = unsafe { std::mem::transmute(val.as_ptr()) };
        }
        rustbus::message::Base::Signature(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            *mutref = unsafe { std::mem::transmute(val.as_ptr()) };
        }
    }
}
