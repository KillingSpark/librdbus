use std::ffi::CStr;

pub const DBUS_MESSAGE_TYPE_INVALID: libc::c_int = 0;
pub const DBUS_MESSAGE_TYPE_METHOD_CALL: libc::c_int = 1;
pub const DBUS_MESSAGE_TYPE_METHOD_RETURN: libc::c_int = 2;
pub const DBUS_MESSAGE_TYPE_ERROR: libc::c_int = 3;
pub const DBUS_MESSAGE_TYPE_SIGNAL: libc::c_int = 4;

#[derive(Clone)]
pub struct DBusMessage {
    pub msg: rustbus::Message,
    refcount: u64,
}

impl DBusMessage {
    pub fn new(msg: rustbus::Message) -> Self {
        Self { msg, refcount: 0 }
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_serial(msg: *const DBusMessage) -> u32 {
    if msg.is_null() {
        -1i32 as u32
    } else {
        let msg = unsafe { &*msg };
        msg.msg.serial.unwrap_or(-1i32 as u32)
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_set_reply_serial(msg: *mut DBusMessage, reply_serial: u32) -> u32 {
    if msg.is_null() {
        0
    } else {
        let msg = unsafe { &mut *msg };
        msg.msg.response_serial = Some(reply_serial);
        1
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_reply_serial(msg: *const DBusMessage) -> u32 {
    if msg.is_null() {
        -1i32 as u32
    } else {
        let msg = unsafe { &*msg };
        msg.msg.response_serial.unwrap_or(-1i32 as u32)
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_sender(msg: *const DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        std::ptr::null()
    } else {
        let msg = unsafe { &*msg };
        if let Some(sender) = &msg.msg.sender {
            unsafe { std::mem::transmute(sender.as_ptr()) }
        } else {
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_new(typ: libc::c_int) -> *mut DBusMessage {
    let mut msg = rustbus::Message::new();
    match typ {
        DBUS_MESSAGE_TYPE_ERROR => msg.typ = rustbus::message::MessageType::Error,
        DBUS_MESSAGE_TYPE_METHOD_CALL => msg.typ = rustbus::message::MessageType::Call,
        DBUS_MESSAGE_TYPE_SIGNAL => msg.typ = rustbus::message::MessageType::Signal,
        DBUS_MESSAGE_TYPE_METHOD_RETURN => msg.typ = rustbus::message::MessageType::Reply,
        _ => msg.typ = rustbus::message::MessageType::Invalid,
    }

    Box::into_raw(Box::new(DBusMessage::new(msg)))
}

#[no_mangle]
pub extern "C" fn dbus_message_new_method_call(
    dest: *const libc::c_char,
    object: *const libc::c_char,
    interface: *const libc::c_char,
    member: *const libc::c_char,
) -> *mut DBusMessage {
    let c_str = unsafe {
        assert!(!dest.is_null());

        CStr::from_ptr(dest)
    };
    let dest = c_str.to_str().unwrap().to_owned();

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

    Box::into_raw(Box::new(DBusMessage::new(
        rustbus::message_builder::MessageBuilder::new()
            .call(member)
            .at(dest)
            .on(object)
            .with_interface(interface)
            .build(),
    )))
}
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

    Box::into_raw(Box::new(DBusMessage::new(
        rustbus::message_builder::MessageBuilder::new()
            .signal(interface, member, object)
            .build(),
    )))
}
#[no_mangle]
pub extern "C" fn dbus_message_new_method_return(call: *const DBusMessage) -> *mut DBusMessage {
    if call.is_null() {
        std::ptr::null_mut()
    } else {
        let call = unsafe { &*call };
        Box::into_raw(Box::new(DBusMessage::new(call.msg.make_response())))
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_new_error(
    call: *const DBusMessage,
    errname: *const libc::c_char,
    errmsg: *const libc::c_char,
) -> *mut DBusMessage {
    if call.is_null() {
        std::ptr::null_mut()
    } else {
        let call = unsafe { &*call };
        let c_str = unsafe {
            assert!(!errname.is_null());
            CStr::from_ptr(errname)
        };
        let errname = c_str.to_str().unwrap().to_owned();
        let c_str = unsafe {
            assert!(!errmsg.is_null());
            CStr::from_ptr(errmsg)
        };
        let errmsg = c_str.to_str().unwrap().to_owned();
        let mut msg = call.msg.make_error_response(errname);
        msg.push_params(vec![errmsg.into()]);
        Box::into_raw(Box::new(DBusMessage::new(msg)))
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_new_error_printf(
    _call: *const DBusMessage,
    _errname: *const libc::c_char,
    _errmsg: *const libc::c_char,
) -> *mut DBusMessage {
    unimplemented!()
}
#[no_mangle]
pub extern "C" fn dbus_message_copy(msg: *const DBusMessage) -> *mut DBusMessage {
    if msg.is_null() {
        std::ptr::null_mut()
    } else {
        let msg = unsafe { &*msg };
        let mut new_msg = msg.clone();
        new_msg.msg.serial = None;
        new_msg.refcount = 1;
        Box::into_raw(Box::new(new_msg))
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_ref(msg: *mut DBusMessage) -> *mut DBusMessage {
    if msg.is_null() {
        std::ptr::null_mut()
    } else {
        let mut_msg = unsafe { &mut *msg };
        mut_msg.refcount += 1;
        msg
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_unref(msg: *mut DBusMessage) {
    if msg.is_null() {
    } else {
        let msg = unsafe { &mut *msg };
        msg.refcount -= 1;
        if msg.refcount == 0 {
            std::mem::drop(unsafe { Box::from_raw(msg) });
        }
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_get_type(msg: *mut DBusMessage) -> libc::c_int {
    if msg.is_null() {
        DBUS_MESSAGE_TYPE_INVALID
    } else {
        let msg = unsafe { &mut *msg };
        match msg.msg.typ {
            rustbus::message::MessageType::Call => DBUS_MESSAGE_TYPE_METHOD_CALL,
            rustbus::message::MessageType::Reply => DBUS_MESSAGE_TYPE_METHOD_RETURN,
            rustbus::message::MessageType::Signal => DBUS_MESSAGE_TYPE_SIGNAL,
            rustbus::message::MessageType::Error => DBUS_MESSAGE_TYPE_ERROR,
            rustbus::message::MessageType::Invalid => DBUS_MESSAGE_TYPE_INVALID,
        }
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_append_args(msg: *mut DBusMessage, typ1: libc::c_int) -> u32 {
    if msg.is_null() {
        0
    } else {
        // TODO is this insanity correct?
        let msg = unsafe { &mut *msg };
        let mut typ_ptr: *const libc::c_int = &typ1;

        // arg pointer always points to directly after the type
        let mut arg_ptr: *const *mut std::ffi::c_void =
            unsafe { std::mem::transmute(typ_ptr.add(1)) };
        loop {
            if typ1 == crate::DBUS_TYPE_INVALID {
                break;
            }
            let typ = unsafe { typ_ptr.read() };

            let param = crate::param_from_parts(typ, unsafe { arg_ptr.read() });
            if let Some(param) = param {
                msg.msg.push_params(vec![param]);
            }
            // move the pointers so that new typ points directly after old arg
            // and new arg points directly after new typ
            typ_ptr = unsafe { std::mem::transmute(arg_ptr.add(1)) };
            arg_ptr = unsafe { std::mem::transmute(typ_ptr.add(1)) };
        }
        1
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_append_args_valist(
    msg: *mut DBusMessage,
    typ1: libc::c_int,
    _va_list: *mut std::ffi::c_void,
) -> u32 {
    dbus_message_append_args(msg, typ1)
}
#[no_mangle]
pub extern "C" fn dbus_message_contains_unix_fds(msg: *mut DBusMessage) -> u32 {
    if msg.is_null() {
        0
    } else {
        // TODO is this insanity correct?
        let msg = unsafe { &mut *msg };
        if msg.msg.raw_fds.is_empty() {
            0
        } else {
            1
        }
    }
}

fn c_to_rustbus_base_type(ctype: libc::c_int) -> Option<rustbus::signature::Base> {
    match ctype {
        crate::DBUS_TYPE_BOOLEAN => Some(rustbus::signature::Base::Boolean),
        crate::DBUS_TYPE_BYTE => Some(rustbus::signature::Base::Byte),
        crate::DBUS_TYPE_INT16 => Some(rustbus::signature::Base::Int16),
        crate::DBUS_TYPE_UINT16 => Some(rustbus::signature::Base::Uint16),
        crate::DBUS_TYPE_INT32 => Some(rustbus::signature::Base::Int32),
        crate::DBUS_TYPE_UINT32 => Some(rustbus::signature::Base::Uint32),
        crate::DBUS_TYPE_INT64 => Some(rustbus::signature::Base::Int64),
        crate::DBUS_TYPE_UINT64 => Some(rustbus::signature::Base::Uint64),
        crate::DBUS_TYPE_DOUBLE => Some(rustbus::signature::Base::Double),
        crate::DBUS_TYPE_UNIXFD => Some(rustbus::signature::Base::UnixFd),
        crate::DBUS_TYPE_STRING => Some(rustbus::signature::Base::String),
        crate::DBUS_TYPE_OBJECTPATH => Some(rustbus::signature::Base::ObjectPath),
        crate::DBUS_TYPE_SIGNATURE => Some(rustbus::signature::Base::Signature),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_args(msg: *mut DBusMessage, typ1: libc::c_int) -> u32 {
    if msg.is_null() {
        0
    } else {
        // TODO is this insanity correct?
        let msg = unsafe { &mut *msg };
        let mut typ_ptr: *const libc::c_int = &typ1;

        // arg pointer always points to directly after the type
        let mut arg_ptr: *const *mut std::ffi::c_void =
            unsafe { std::mem::transmute(typ_ptr.add(1)) };

        let mut counter = 0;
        loop {
            if typ1 == crate::DBUS_TYPE_INVALID {
                break;
            }
            if counter >= msg.msg.params.len() {
                return 0;
            }
            let typ = unsafe { typ_ptr.read() };
            if let Some(base_type) = c_to_rustbus_base_type(typ) {
                let param = &msg.msg.params[counter];
                if rustbus::signature::Type::Base(base_type) == param.sig() {
                    if let rustbus::message::Param::Base(base_param) = param {
                        crate::write_base_param(base_param, unsafe { arg_ptr.read() });
                    }
                } else {
                    // TODO What do we do here?!
                    unimplemented!();
                }
                // move the pointers so that new typ points directly after old arg
                // and new arg points directly after new typ
                typ_ptr = unsafe { std::mem::transmute(arg_ptr.add(1)) };
                arg_ptr = unsafe { std::mem::transmute(typ_ptr.add(1)) };
                counter += 1;
            } else {
                if typ != crate::DBUS_TYPE_ARRAY {
                    return 0;
                }
                let element_type_ptr: *const libc::c_int =
                    unsafe { std::mem::transmute(arg_ptr.add(1)) };
                let element_type = unsafe { element_type_ptr.read() };
                arg_ptr = unsafe { std::mem::transmute(element_type_ptr.add(1)) };
                let array_size_ptr: *const u32 = unsafe { std::mem::transmute(arg_ptr.add(1)) };

                if let Some(base_type) = c_to_rustbus_base_type(element_type) {
                    let array_size = unsafe { array_size_ptr.read() };
                    if let rustbus::message::Param::Container(rustbus::message::Container::Array(
                        array_param,
                    )) = &msg.msg.params[counter]
                    {
                        for idx in 0..u32::min(array_size, array_param.values.len() as u32) {
                            let param = &array_param.values[idx as usize];
                            if rustbus::signature::Type::Base(base_type) == param.sig() {
                                if let rustbus::message::Param::Base(base_param) = param {
                                    crate::write_base_param(base_param, unsafe { arg_ptr.read() });
                                }
                            } else {
                                // TODO What do we do here?!
                                unimplemented!();
                            }
                        }
                    } else {
                        // TODO What do we do here?!
                        unimplemented!();
                    }
                } else {
                    return 0;
                }

                typ_ptr = unsafe { std::mem::transmute(array_size_ptr.add(1)) };
                arg_ptr = unsafe { std::mem::transmute(typ_ptr.add(1)) };
            }
        }
        1
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_args_valist(
    msg: *mut DBusMessage,
    typ1: libc::c_int,
    _va_list: *mut std::ffi::c_void,
) -> u32 {
    dbus_message_get_args(msg, typ1)
}
