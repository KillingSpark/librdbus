#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

mod bus;
mod connection;
mod data_slot;
mod error;
mod message;
mod message_iter;
mod private;
mod validate;
use message::*;
use rustbus::params;
use std::ffi::CStr;

use libc;

#[repr(C)]
#[derive(Debug)]
pub enum DBusBusType {
    DBUS_BUS_SESSION,
    DBUS_BUS_SYSTEM,
    DBUS_BUS_STARTER,
}

#[repr(C)]
#[derive(Debug)]
pub enum DBusHandlerResult {
    DBUS_HANDLER_RESULT_HANDLED,
    DBUS_HANDLER_RESULT_NOT_YET_HANDLED,
    DBUS_HANDLER_RESULT_NEED_MEMORY,
}

pub type DBusFreeFunction = extern "C" fn(*mut std::ffi::c_void);

pub type DBusHandleMessageFunction = fn(
    *mut connection::DBusConnection,
    *mut DBusMessage,
    *mut std::ffi::c_void,
) -> DBusHandlerResult;

const METHOD_CALL_STR: &'static str = "method_call";
const METHOD_RETURN_STR: &'static str = "method_return";
const SIGNAL_STR: &'static str = "signal";
const ERROR_STR: &'static str = "error";
const INVALID_STR: &'static str = "invalid";

#[no_mangle]
pub extern "C" fn dbus_message_type_from_string(typ: *const libc::c_char) -> libc::c_int {
    if typ.is_null() {
        return DBUS_MESSAGE_TYPE_INVALID;
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(typ) };

    if cstr.to_bytes().eq(METHOD_CALL_STR.as_bytes()) {
        return DBUS_MESSAGE_TYPE_METHOD_CALL;
    }
    if cstr.to_bytes().eq(METHOD_RETURN_STR.as_bytes()) {
        return DBUS_MESSAGE_TYPE_METHOD_RETURN;
    }
    if cstr.to_bytes().eq(SIGNAL_STR.as_bytes()) {
        return DBUS_MESSAGE_TYPE_SIGNAL;
    }
    if cstr.to_bytes().eq(ERROR_STR.as_bytes()) {
        return DBUS_MESSAGE_TYPE_ERROR;
    }
    return DBUS_MESSAGE_TYPE_INVALID;
}

#[no_mangle]
pub extern "C" fn dbus_message_type_to_string(typ: libc::c_int) -> *const libc::c_char {
    unsafe {
        std::mem::transmute(match typ {
            DBUS_MESSAGE_TYPE_METHOD_CALL => METHOD_CALL_STR.as_ptr(),
            DBUS_MESSAGE_TYPE_METHOD_RETURN => METHOD_RETURN_STR.as_ptr(),
            DBUS_MESSAGE_TYPE_SIGNAL => SIGNAL_STR.as_ptr(),
            DBUS_MESSAGE_TYPE_ERROR => ERROR_STR.as_ptr(),
            _ => INVALID_STR.as_ptr(),
        })
    }
}

pub type DbusBool = u32;
pub fn dbus_bool(b: bool) -> DbusBool {
    if b {
        1
    } else {
        0
    }
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

#[no_mangle]
pub extern "C" fn dbus_malloc(size: libc::size_t) -> *mut std::ffi::c_void {
    if size == 0 {
        std::ptr::null_mut()
    } else {
        unsafe { libc::malloc(size) }
    }
}

#[no_mangle]
pub extern "C" fn dbus_free(data: *mut std::ffi::c_void) {
    unsafe { libc::free(data) }
}

#[no_mangle]
pub extern "C" fn dbus_bus_add_match(
    con: *mut connection::DBusConnection,
    rule: *const libc::c_char,
    err: *mut error::DBusError,
) {
    if con.is_null() {
        return;
    }
    let con = unsafe { &mut *con };

    let c_str = unsafe {
        assert!(!rule.is_null());
        std::ffi::CStr::from_ptr(rule)
    };
    let rule = c_str.to_str().unwrap();
    let mut msg = rustbus::standard_messages::add_match(rule.to_owned());
    con.con.send_message(&mut msg, None).unwrap();

    // TODO set error
}

pub fn param_from_parts<'a>(
    argtyp: libc::c_int,
    arg: *mut std::ffi::c_void,
) -> Option<params::Param<'a, 'a>> {
    let param: params::Param = match argtyp {
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
            params::Base::ObjectPath(arg).into()
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
            params::Base::ObjectPath(arg).into()
        }
        DBUS_TYPE_INT16 => {
            assert!(!arg.is_null());
            let ptr: *const i16 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Int16(val).into()
        }
        DBUS_TYPE_UINT16 => {
            assert!(!arg.is_null());
            let ptr: *const u16 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Uint16(val).into()
        }
        DBUS_TYPE_INT32 => {
            assert!(!arg.is_null());
            let ptr: *const i32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Int32(val).into()
        }
        DBUS_TYPE_UINT32 => {
            assert!(!arg.is_null());
            let ptr: *const u32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Uint32(val).into()
        }
        DBUS_TYPE_INT64 => {
            assert!(!arg.is_null());
            let ptr: *const i64 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Int64(val).into()
        }
        DBUS_TYPE_UINT64 => {
            assert!(!arg.is_null());
            let ptr: *const u64 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Uint64(val).into()
        }
        DBUS_TYPE_BOOLEAN => {
            assert!(!arg.is_null());
            let ptr: *const u32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Boolean(val != 0).into()
        }
        DBUS_TYPE_BYTE => {
            assert!(!arg.is_null());
            let ptr: *const u8 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Byte(val).into()
        }
        DBUS_TYPE_DOUBLE => {
            assert!(!arg.is_null());
            let ptr: *const u64 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::Double(val).into()
        }
        DBUS_TYPE_UNIXFD => {
            assert!(!arg.is_null());
            let ptr: *const u32 = unsafe { std::mem::transmute(arg) };
            let val = unsafe { ptr.read() };
            params::Base::UnixFd(val).into()
        }
        _ => return None,
    };
    Some(param)
}

pub fn write_base_param<'a>(
    param: &params::Base<'a>,
    string_arena: &mut crate::StringArena,
    arg: *mut std::ffi::c_void,
) {
    match param {
        params::Base::Boolean(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = if *val { 1 } else { 0 };
        }
        params::Base::Byte(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u8 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Int16(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i16 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Uint16(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u16 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Int32(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i32 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Uint32(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Int64(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i64 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Uint64(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u64 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::Double(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u64 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }
        params::Base::UnixFd(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = *val;
        }

        params::Base::String(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };

            let cstr = crate::get_cstring(string_arena, &val);
            *mutref = unsafe { std::mem::transmute(cstr.as_ptr()) };
        }
        params::Base::ObjectPath(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            let cstr = crate::get_cstring(string_arena, &val);
            *mutref = unsafe { std::mem::transmute(cstr.as_ptr()) };
        }
        params::Base::Signature(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            let cstr = crate::get_cstring(string_arena, &val);
            *mutref = unsafe { std::mem::transmute(cstr.as_ptr()) };
        }
        params::Base::BooleanRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = if **val { 1 } else { 0 };
        }
        params::Base::ByteRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u8 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::Int16Ref(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i16 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::Uint16Ref(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u16 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::Int32Ref(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i32 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::Uint32Ref(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::Int64Ref(val) => {
            assert!(!arg.is_null());
            let mutref: &mut i64 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::Uint64Ref(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u64 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::DoubleRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u64 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }
        params::Base::UnixFdRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut u32 = unsafe { std::mem::transmute(arg) };
            *mutref = **val;
        }

        params::Base::StringRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };

            let cstr = crate::get_cstring(string_arena, &val);
            *mutref = unsafe { std::mem::transmute(cstr.as_ptr()) };
        }
        params::Base::ObjectPathRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            let cstr = crate::get_cstring(string_arena, &val);
            *mutref = unsafe { std::mem::transmute(cstr.as_ptr()) };
        }
        params::Base::SignatureRef(val) => {
            assert!(!arg.is_null());
            let mutref: &mut *const libc::c_char = unsafe { std::mem::transmute(arg) };
            let cstr = crate::get_cstring(string_arena, &val);
            *mutref = unsafe { std::mem::transmute(cstr.as_ptr()) };
        }
    }
}
