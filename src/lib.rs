#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use libc;
pub use rustbus::*;

#[repr(C)]
#[derive(Debug)]
pub enum DBusBusType {
    DBUS_BUS_SESSION,
    DBUS_BUS_SYSTEM,
    DBUS_BUS_STARTER,
}

type DBusConnection = rustbus::client_conn::RpcConn;
type DBusMessage = rustbus::Message;

pub struct Error {
    error: Option<String>,
}

#[no_mangle]
pub extern "C" fn dbus_error_new() -> *mut Error {
    Box::into_raw(Box::new(Error { error: None }))
}

#[no_mangle]
pub extern "C" fn dbus_error_is_set(err: *mut Error) -> libc::c_int {
    if err.is_null() {
        return 0;
    }

    let err: &mut Error = unsafe { &mut *err };
    if err.error.is_some() {
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
            err.error = Some(format!("Unknown bus type: {:?}", bus));
            return std::ptr::null_mut();
        }
    };
    match path {
        Ok(path) => match rustbus::client_conn::Conn::connect_to_bus(path, false) {
            Ok(con) => Box::into_raw(Box::new(rustbus::client_conn::RpcConn::new(con))),
            Err(e) => {
                if !err.is_null() {
                    let err: &mut Error = unsafe { &mut *err };
                    err.error = Some(format!("Could not connect to bus: {:?}", e));
                }
                std::ptr::null_mut()
            }
        },
        Err(e) => {
            if !err.is_null() {
                let err: &mut Error = unsafe { &mut *err };
                err.error = Some(format!("Could open path for bus: {:?}", e));
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
                err.error = Some(format!("Error sending message: {:?}", e));
            }
        }
    }
}
use std::ffi::CStr;

#[no_mangle]
pub extern "C" fn dbus_message_new_signal(
    object: *const libc::c_char,
    interface: *const libc::c_char,
    member: *const libc::c_char,
) -> *mut DBusMessage {
    let c_str = unsafe {
        assert!(!object.is_null());

        CStr::from_ptr(object)
    };
    let object = c_str.to_str().unwrap().to_owned();

    let c_str = unsafe {
        assert!(!interface.is_null());

        CStr::from_ptr(interface)
    };
    let interface = c_str.to_str().unwrap().to_owned();

    let c_str = unsafe {
        assert!(!member.is_null());

        CStr::from_ptr(member)
    };
    let member = c_str.to_str().unwrap().to_owned();

    Box::into_raw(Box::new(
        rustbus::message_builder::MessageBuilder::new()
            .signal(interface, member, object)
            .build(),
    ))
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
    if let Err(e) = con.send_message(msg.clone(), None) {
        if !err.is_null() {
            let err = unsafe { &mut *err };
            err.error = Some(format!("Error sending message: {:?}", e));
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

pub struct SubIter {
    params: Vec<rustbus::message::Param>,
    typ: rustbus::signature::Container,
}

enum MessageIterInternal {
    // pushes contents into message
    MainIter(*mut DBusMessage),
    // pushes contents into parent when closed
    SubIter(SubIter),
}

#[repr(C)]
pub struct DBusMessageIter {
    inner: *mut MessageIterInternal,
}

impl DBusMessageIter {
    fn append(&mut self, param: rustbus::message::Param) {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainIter(msg) => {
                let msg = unsafe { &mut **msg };
                msg.push_params(vec![param]);
            }
            MessageIterInternal::SubIter(sub) => {
                sub.params.push(param);
            }
        }
    }

    fn close(&mut self, parent: &mut DBusMessageIter) {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainIter(_msg) => {
                // nothing to do here
            }
            MessageIterInternal::SubIter(sub) => match &sub.typ {
                rustbus::signature::Container::Array(sig) => parent.append(
                    rustbus::message::Container::Array(rustbus::message::Array {
                        element_sig: sig.as_ref().clone(),
                        values: sub.params.clone(),
                    })
                    .into(),
                ),
                rustbus::signature::Container::Dict(_, _) => unimplemented!(),
                rustbus::signature::Container::Variant => parent.append(
                    rustbus::message::Container::Variant(Box::new(rustbus::message::Variant {
                        sig: sub.params[0].sig(),
                        value: sub.params[0].clone(),
                    }))
                    .into(),
                ),
                rustbus::signature::Container::Struct(_sigs) => {
                    parent.append(rustbus::message::Container::Struct(sub.params.clone()).into())
                }
            },
        }
        std::mem::drop(unsafe { Box::from_raw(self.inner) });
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_init_append(msg: *mut DBusMessage, args: *mut DBusMessageIter) {
    if args.is_null() {
        return;
    }
    let args = unsafe { &mut *args };
    *args = DBusMessageIter {
        inner: Box::into_raw(Box::new(MessageIterInternal::MainIter(msg))),
    };
}

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
pub extern "C" fn dbus_message_iter_append_basic(
    args: *mut DBusMessageIter,
    argtyp: libc::c_int,
    arg: *mut std::ffi::c_void,
) {
    if args.is_null() {
        return;
    }
    let args = unsafe { &mut *args };

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
        _ => return,
    };
    args.append(param);
}

pub extern "C" fn dbus_message_iter_open_container(
    parent: *mut DBusMessageIter,
    argtyp: libc::c_int,
    argsig: *const libc::c_char,
    sub: *mut DBusMessageIter,
) {
    if parent.is_null() {
        return;
    }
    if sub.is_null() {
        return;
    }
    let sub = unsafe { &mut *sub };

    let c_str = unsafe {
        assert!(!argsig.is_null());
        let arg: *const *const libc::c_char = std::mem::transmute(argsig);
        let arg = arg.read();
        assert!(!arg.is_null());
        CStr::from_ptr(arg)
    };
    let argsig = c_str.to_str().unwrap();
    let mut argsig = rustbus::signature::Type::parse_description(argsig).unwrap();
    let typ = match argtyp {
        DBUS_TYPE_ARRAY => rustbus::signature::Container::Array(Box::new(argsig.remove(0))),
        DBUS_TYPE_STRUCT => rustbus::signature::Container::Struct(argsig),
        DBUS_TYPE_VARIANT => rustbus::signature::Container::Variant,
        _ => return,
    };

    *sub = DBusMessageIter {
        inner: Box::into_raw(Box::new(MessageIterInternal::SubIter(SubIter {
            params: Vec::new(),
            typ,
        }))),
    }
}

pub extern "C" fn dbus_message_iter_close_container(
    parent: *mut DBusMessageIter,
    sub: *mut DBusMessageIter,
) {
    let parent = unsafe { &mut *parent };
    let sub = unsafe { &mut *sub };
    sub.close(parent);
}
