use crate::dbus_bool;
use rustbus::params;
use std::ffi::CStr;

pub const DBUS_MESSAGE_TYPE_INVALID: libc::c_int = 0;
pub const DBUS_MESSAGE_TYPE_METHOD_CALL: libc::c_int = 1;
pub const DBUS_MESSAGE_TYPE_METHOD_RETURN: libc::c_int = 2;
pub const DBUS_MESSAGE_TYPE_ERROR: libc::c_int = 3;
pub const DBUS_MESSAGE_TYPE_SIGNAL: libc::c_int = 4;

pub type StringArena = std::collections::HashMap<String, Box<std::ffi::CString>>;

pub fn get_cstring<'a>(arena: &'a mut StringArena, string: &str) -> &'a std::ffi::CString {
    if !arena.contains_key(string) {
        arena.insert(
            string.to_owned(),
            Box::new(std::ffi::CString::new(string).unwrap()),
        );
    }
    arena.get(string).unwrap()
}

#[derive(Clone)]
pub struct DBusMessage<'a> {
    pub msg: rustbus::Message<'a, 'a>,
    refcount: u64,
    pub string_arena: StringArena,
    locked: bool,
    pub app_data: Vec<crate::data_slot::AppData>,
    buffer: Vec<u8>,
}

impl<'a> DBusMessage<'a> {
    pub fn new(msg: rustbus::Message<'a, 'a>) -> Self {
        Self {
            msg,
            refcount: 0,
            string_arena: std::collections::HashMap::new(),
            locked: false,
            app_data: Vec::new(),
            buffer: Vec::new(),
        }
    }

    fn finalize(&mut self) {
        self.app_data.clear();
    }
}

impl<'a> Drop for DBusMessage<'a> {
    fn drop(&mut self) {
        self.finalize();
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
pub extern "C" fn dbus_message_new<'a>(typ: libc::c_int) -> *mut DBusMessage<'a> {
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
pub extern "C" fn dbus_message_new_method_call<'a>(
    dest: *const libc::c_char,
    object: *const libc::c_char,
    interface: *const libc::c_char,
    member: *const libc::c_char,
) -> *mut DBusMessage<'a> {
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
pub extern "C" fn dbus_message_new_signal<'a>(
    object: *const libc::c_char,
    interface: *const libc::c_char,
    member: *const libc::c_char,
) -> *mut DBusMessage<'a> {
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
pub extern "C" fn dbus_message_new_error<'a>(
    call: *const DBusMessage<'a>,
    errname: *const libc::c_char,
    errmsg: *const libc::c_char,
) -> *mut DBusMessage<'a> {
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
        let msg = call.msg.make_error_response(errname, Some(errmsg));
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

pub fn c_to_rustbus_base_type(ctype: libc::c_int) -> Option<rustbus::signature::Base> {
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

pub fn rustbus_to_c_base_type(rtype: &rustbus::signature::Base) -> libc::c_int {
    match rtype {
        rustbus::signature::Base::Boolean => crate::DBUS_TYPE_BOOLEAN,
        rustbus::signature::Base::Byte => crate::DBUS_TYPE_BYTE,
        rustbus::signature::Base::Int16 => crate::DBUS_TYPE_INT16,
        rustbus::signature::Base::Uint16 => crate::DBUS_TYPE_UINT16,
        rustbus::signature::Base::Int32 => crate::DBUS_TYPE_INT32,
        rustbus::signature::Base::Uint32 => crate::DBUS_TYPE_UINT32,
        rustbus::signature::Base::Int64 => crate::DBUS_TYPE_INT64,
        rustbus::signature::Base::Uint64 => crate::DBUS_TYPE_UINT64,
        rustbus::signature::Base::Double => crate::DBUS_TYPE_DOUBLE,
        rustbus::signature::Base::UnixFd => crate::DBUS_TYPE_UNIXFD,
        rustbus::signature::Base::String => crate::DBUS_TYPE_STRING,
        rustbus::signature::Base::ObjectPath => crate::DBUS_TYPE_OBJECTPATH,
        rustbus::signature::Base::Signature => crate::DBUS_TYPE_SIGNATURE,
    }
}
pub fn rustbus_to_c_container_type(rtype: &rustbus::signature::Container) -> libc::c_int {
    match rtype {
        rustbus::signature::Container::Array(_) => crate::DBUS_TYPE_ARRAY,
        rustbus::signature::Container::Dict(_, _) => crate::DBUS_TYPE_ARRAY,
        rustbus::signature::Container::Struct(_) => crate::DBUS_TYPE_STRUCT,
        rustbus::signature::Container::Variant => crate::DBUS_TYPE_VARIANT,
    }
}
pub fn rustbus_to_c_type(rtype: &rustbus::signature::Type) -> libc::c_int {
    match rtype {
        rustbus::signature::Type::Base(b) => rustbus_to_c_base_type(b),
        rustbus::signature::Type::Container(c) => rustbus_to_c_container_type(c),
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_get_args<'a>(msg: *mut DBusMessage<'a>, typ1: libc::c_int) -> u32 {
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
                    if let params::Param::Base(base_param) = param {
                        crate::write_base_param(base_param, &mut msg.string_arena, unsafe {
                            arg_ptr.read()
                        });
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
                    if let params::Param::Container(params::Container::Array(array_param)) =
                        &msg.msg.params[counter]
                    {
                        for idx in 0..u32::min(array_size, array_param.values.len() as u32) {
                            let param = &array_param.values[idx as usize];
                            if rustbus::signature::Type::Base(base_type) == param.sig() {
                                if let params::Param::Base(base_param) = param {
                                    crate::write_base_param(
                                        base_param,
                                        &mut msg.string_arena,
                                        unsafe { arg_ptr.read() },
                                    );
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

#[no_mangle]
pub extern "C" fn dbus_message_set_no_reply(_msg: *mut crate::DBusMessage) {
    unimplemented!();
}
#[no_mangle]
pub extern "C" fn dbus_message_get_no_reply(_msg: *mut crate::DBusMessage) {
    unimplemented!();
}
#[no_mangle]
pub extern "C" fn dbus_message_set_auto_start(_msg: *mut crate::DBusMessage) {
    unimplemented!();
}
#[no_mangle]
pub extern "C" fn dbus_message_get_auto_start(_msg: *mut crate::DBusMessage) {
    unimplemented!();
}
#[no_mangle]
pub extern "C" fn dbus_message_set_allow_interactive_authorization(_msg: *mut crate::DBusMessage) {
    unimplemented!();
}
#[no_mangle]
pub extern "C" fn dbus_message_get_allow_interactive_authorization(_msg: *mut crate::DBusMessage) {
    unimplemented!();
}
#[no_mangle]
pub extern "C" fn dbus_message_get_path(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    if let Some(s) = &msg.msg.object {
        let cstr = crate::get_cstring(&mut msg.string_arena, &s);
        unsafe { std::mem::transmute(cstr.as_ptr()) }
    } else {
        std::ptr::null()
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_set_path(
    msg: *mut crate::DBusMessage,
    path: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!path.is_null());
        CStr::from_ptr(path)
    };
    let path = c_str.to_str().unwrap();

    msg.msg.object = Some(path.to_owned());
    1
}
#[no_mangle]
pub extern "C" fn dbus_message_has_path(
    msg: *mut crate::DBusMessage,
    path: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!path.is_null());
        CStr::from_ptr(path)
    };
    let path = c_str.to_str().unwrap();

    dbus_bool(
        msg.msg
            .object
            .as_ref()
            .map(|d| path.eq(d.as_str()))
            .unwrap_or(false),
    )
}
#[no_mangle]
pub extern "C" fn dbus_message_get_path_decomposed(
    msg: *mut crate::DBusMessage,
    output: *mut *const *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &*msg };

    if let Some(object) = &msg.msg.object {
        let mut ptr_array = Vec::new();
        for element in object.split('/') {
            let cstr = std::ffi::CString::new(element).unwrap();
            ptr_array.push(cstr.as_ptr());
            std::mem::forget(cstr);
        }
        ptr_array.push(std::ptr::null());

        let boxed = Box::new(ptr_array);
        let array_ptr = boxed.as_ref().as_ptr();
        // forget box. Needs to be freed in dbus_free_string_array()
        let _ptr = Box::into_raw(boxed);
        unsafe { *output = array_ptr };
    }
    1
}

#[no_mangle]
pub extern "C" fn dbus_message_get_interface(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    if let Some(s) = &msg.msg.interface {
        let cstr = crate::get_cstring(&mut msg.string_arena, &s);
        unsafe { std::mem::transmute(cstr.as_ptr()) }
    } else {
        std::ptr::null()
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_set_interface(
    msg: *mut crate::DBusMessage,
    interface: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!interface.is_null());
        CStr::from_ptr(interface)
    };
    let path = c_str.to_str().unwrap();

    msg.msg.interface = Some(path.to_owned());
    1
}
#[no_mangle]
pub extern "C" fn dbus_message_has_interface(
    msg: *mut crate::DBusMessage,
    interface: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!interface.is_null());
        CStr::from_ptr(interface)
    };
    let interface = c_str.to_str().unwrap();

    if msg.msg.interface == Some(interface.to_owned()) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_member(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    if let Some(s) = &msg.msg.member {
        let cstr = crate::get_cstring(&mut msg.string_arena, &s);
        unsafe { std::mem::transmute(cstr.as_ptr()) }
    } else {
        std::ptr::null()
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_set_member(
    msg: *mut crate::DBusMessage,
    member: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!member.is_null());
        CStr::from_ptr(member)
    };
    let path = c_str.to_str().unwrap();

    msg.msg.member = Some(path.to_owned());
    1
}
#[no_mangle]
pub extern "C" fn dbus_message_has_member(
    msg: *mut crate::DBusMessage,
    member: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!member.is_null());
        CStr::from_ptr(member)
    };
    let member = c_str.to_str().unwrap();

    if msg.msg.member == Some(member.to_owned()) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_get_error_name(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    if let Some(s) = &msg.msg.error_name {
        let cstr = crate::get_cstring(&mut msg.string_arena, &s);
        unsafe { std::mem::transmute(cstr.as_ptr()) }
    } else {
        std::ptr::null()
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_set_error_name(
    msg: *mut crate::DBusMessage,
    error_name: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!error_name.is_null());
        CStr::from_ptr(error_name)
    };
    let error_name = c_str.to_str().unwrap();

    msg.msg.member = Some(error_name.to_owned());
    1
}

#[no_mangle]
pub extern "C" fn dbus_message_get_destination(
    msg: *mut crate::DBusMessage,
) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    if let Some(s) = &msg.msg.destination {
        let cstr = crate::get_cstring(&mut msg.string_arena, &s);
        unsafe { std::mem::transmute(cstr.as_ptr()) }
    } else {
        std::ptr::null()
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_set_destination(
    msg: *mut crate::DBusMessage,
    destination: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!destination.is_null());
        CStr::from_ptr(destination)
    };
    let destination = c_str.to_str().unwrap();

    msg.msg.destination = Some(destination.to_owned());
    1
}
#[no_mangle]
pub extern "C" fn dbus_message_get_sender(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    if let Some(s) = &msg.msg.sender {
        let cstr = crate::get_cstring(&mut msg.string_arena, &s);
        unsafe { std::mem::transmute(cstr.as_ptr()) }
    } else {
        std::ptr::null()
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_set_sender(
    msg: *mut crate::DBusMessage,
    sender: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!sender.is_null());
        CStr::from_ptr(sender)
    };
    let sender = c_str.to_str().unwrap();

    msg.msg.sender = Some(sender.to_owned());
    1
}
#[no_mangle]
pub extern "C" fn dbus_message_get_signature(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &mut *msg };

    let mut sig_str = String::new();
    for t in msg.msg.sig() {
        t.to_str(&mut sig_str);
    }

    let cstr = crate::get_cstring(&mut msg.string_arena, &sig_str);
    unsafe { std::mem::transmute(cstr.as_ptr()) }
}
#[no_mangle]
pub extern "C" fn dbus_message_is_method_call(msg: *mut crate::DBusMessage) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    dbus_bool(if let rustbus::MessageType::Call = msg.msg.typ {
        true
    } else {
        false
    })
}
#[no_mangle]
pub extern "C" fn dbus_message_is_signal(msg: *mut crate::DBusMessage) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    dbus_bool(if let rustbus::MessageType::Signal = msg.msg.typ {
        true
    } else {
        false
    })
}
#[no_mangle]
pub extern "C" fn dbus_message_is_error(msg: *mut crate::DBusMessage) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    dbus_bool(if let rustbus::MessageType::Error = msg.msg.typ {
        true
    } else {
        false
    })
}
#[no_mangle]
pub extern "C" fn dbus_message_is_reply(msg: *mut crate::DBusMessage) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    dbus_bool(if let rustbus::MessageType::Reply = msg.msg.typ {
        true
    } else {
        false
    })
}
#[no_mangle]
pub extern "C" fn dbus_message_has_destination(
    msg: *mut crate::DBusMessage,
    destination: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!destination.is_null());
        CStr::from_ptr(destination)
    };
    let destination = c_str.to_str().unwrap();

    dbus_bool(
        msg.msg
            .destination
            .as_ref()
            .map(|d| destination.eq(d.as_str()))
            .unwrap_or(false),
    )
}

#[no_mangle]
pub extern "C" fn dbus_message_has_sender(
    msg: *mut crate::DBusMessage,
    sender: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!sender.is_null());
        CStr::from_ptr(sender)
    };
    let sender = c_str.to_str().unwrap();

    dbus_bool(
        msg.msg
            .sender
            .as_ref()
            .map(|d| sender.eq(d.as_str()))
            .unwrap_or(false),
    )
}
#[no_mangle]
pub extern "C" fn dbus_message_has_signature(
    msg: *mut crate::DBusMessage,
    sig: *const libc::c_char,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    let c_str = unsafe {
        assert!(!sig.is_null());
        CStr::from_ptr(sig)
    };
    let sig = c_str.to_str().unwrap();

    let mut sig_str = String::new();
    for t in msg.msg.sig() {
        t.to_str(&mut sig_str);
    }

    dbus_bool(sig.eq(&sig_str))
}
#[no_mangle]
pub extern "C" fn dbus_set_error_from_message(
    err: *mut crate::DBusError,
    msg: *mut crate::DBusMessage,
) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };
    if err.is_null() {
        return 0;
    }
    let err = unsafe { &mut *err };

    dbus_bool(if let rustbus::MessageType::Error = msg.msg.typ {
        err.is_set = true;
        err.name = Box::new(msg.msg.error_name.clone().unwrap_or("".to_owned()));
        if !msg.msg.params.is_empty() {
            if let rustbus::params::Param::Base(rustbus::params::Base::String(error_msg)) =
                &msg.msg.params[0]
            {
                err.error = Box::new(error_msg.clone());
            }
            if let rustbus::params::Param::Base(rustbus::params::Base::StringRef(error_msg)) =
                &msg.msg.params[0]
            {
                err.error = Box::new(error_msg.to_string());
            }
        }
        true
    } else {
        false
    })
}

#[no_mangle]
pub extern "C" fn dbus_message_contains_unix_fds(msg: *mut crate::DBusMessage) -> u32 {
    if msg.is_null() {
        return 0;
    }
    let msg = unsafe { &mut *msg };

    dbus_bool(!msg.msg.raw_fds.is_empty())
}

#[no_mangle]
pub extern "C" fn dbus_message_set_container_instance(
    _msg: *mut crate::DBusMessage,
    _object: *const libc::c_char,
) -> u32 {
    unimplemented!()
}
#[no_mangle]
pub extern "C" fn dbus_message_get_container_instance(
    _msg: *mut crate::DBusMessage,
) -> *const libc::c_char {
    unimplemented!()
}
#[no_mangle]
pub extern "C" fn dbus_message_set_serial(msg: *mut crate::DBusMessage, serial: u32) {
    if msg.is_null() {
        return;
    }
    let msg = unsafe { &mut *msg };

    msg.msg.serial = Some(serial);
}
#[no_mangle]
pub extern "C" fn dbus_message_lock(msg: *mut crate::DBusMessage) {
    if msg.is_null() {
        return;
    }
    let msg = unsafe { &mut *msg };
    msg.locked = true;
}

#[no_mangle]
pub extern "C" fn dbus_message_marshal(
    msg: *mut crate::DBusMessage,
    dest: *mut *const libc::c_char,
    len: *mut libc::c_int,
) -> u32 {
    if msg.is_null() {
        return dbus_bool(false);
    }
    let msg = unsafe { &mut *msg };
    if dest.is_null() {
        return dbus_bool(false);
    }
    let dest = unsafe { &mut *dest };
    if len.is_null() {
        return dbus_bool(false);
    }
    let len = unsafe { &mut *len };

    // TODO make a buffer pool or something similar
    msg.buffer.clear();
    match rustbus::wire::marshal::marshal(
        &msg.msg,
        rustbus::message::ByteOrder::LittleEndian,
        &[],
        &mut msg.buffer,
    ) {
        Ok(()) => {
            *dest = unsafe { std::mem::transmute(msg.buffer.as_ptr()) };
            *len = msg.buffer.len() as libc::c_int;
            dbus_bool(true)
        }
        Err(_) => dbus_bool(false),
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_demarshal<'a>(
    source: *mut *const libc::c_char,
    len: libc::c_int,
    err: *mut crate::DBusError,
) -> *mut DBusMessage<'a> {
    // needs to be mem::forget'd before exiting, the memory should not be freed by this function
    let buf = unsafe { Vec::from_raw_parts(source as *mut u8, len as usize, len as usize) };

    let (header_bytes, header) = match rustbus::wire::unmarshal::unmarshal_header(&buf, 0) {
        Ok(h) => h,
        Err(e) => {
            if err.is_null() {
                std::mem::forget(buf);
                return std::ptr::null_mut();
            }
            let err = unsafe { &mut *err };
            err.name = Box::new("DemarshallingError".to_owned());
            err.error = Box::new(format!("Demarshalling error: {:?}", e));
            err.is_set = true;
            std::mem::forget(buf);
            return std::ptr::null_mut();
        }
    };

    match rustbus::wire::unmarshal::unmarshal_next_message(&header, &buf, header_bytes) {
        Ok((_bytes, msg)) => {
            let msg = Box::new(DBusMessage::new(msg));
            std::mem::forget(buf);
            return Box::into_raw(msg);
        }
        Err(e) => {
            if err.is_null() {
                std::mem::forget(buf);
                return std::ptr::null_mut();
            }
            let err = unsafe { &mut *err };
            err.name = Box::new("DemarshallingError".to_owned());
            err.error = Box::new(format!("Demarshalling error: {:?}", e));
            err.is_set = true;
            std::mem::forget(buf);
            return std::ptr::null_mut();
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_demarshal_bytes_needed(
    source: *mut *const libc::c_char,
    len: libc::c_int,
    err: *mut crate::DBusError,
) -> libc::c_int {
    // needs to be mem::forget'd before exiting, the memory should not be freed by this function
    let buf = unsafe { Vec::from_raw_parts(source as *mut u8, len as usize, len as usize) };
    let (header_bytes, header) = match rustbus::wire::unmarshal::unmarshal_header(&buf, 0) {
        Ok(h) => h,
        Err(e) => {
            let ret = match e {
                rustbus::wire::unmarshal::Error::NotEnoughBytes => 0,
                _ => -1,
            };
            if err.is_null() {
                std::mem::forget(buf);
                return ret;
            }
            let err = unsafe { &mut *err };
            err.name = Box::new("DemarshallingError".to_owned());
            err.error = Box::new(format!("Demarshalling error: {:?}", e));
            err.is_set = true;
            std::mem::forget(buf);
            return ret;
        }
    };

    // have the body len in the header but still need the len of the header fields
    if buf.len() < header_bytes + 4 {
        std::mem::forget(buf);
        return 0;
    }
    let (_, header_field_len) =
        rustbus::wire::util::parse_u32(&buf[header_bytes..], header.byteorder).unwrap();

    std::mem::forget(buf);
    (header.body_len + header_field_len + header_bytes as u32) as libc::c_int
}
