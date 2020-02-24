use std::ffi::CStr;

pub struct SubAppendIter {
    params: Vec<rustbus::message::Param>,
    typ: rustbus::signature::Container,
}

enum MessageIterInternal {
    // pushes contents into message
    MainAppendIter(*mut crate::DBusMessage),
    // pushes contents into parent when closed
    SubAppendIter(SubAppendIter),
    MainIter(*const crate::DBusMessage),
    StructIter(*const Vec<rustbus::message::Param>),
    DictIter(*const rustbus::message::Dict),
    ArrayIter(*const rustbus::message::Array),
    VariantIter(*const rustbus::message::Variant),
    DictEntryIter(
        *const rustbus::message::Base,
        *const rustbus::message::Param,
    ),
}

#[repr(C)]
pub struct DBusMessageIter {
    inner: *mut MessageIterInternal,
    counter: usize,
}

#[derive(Debug)]
enum RustbusTypeOrDictEntry {
    Rustbus(rustbus::signature::Type),
    DictEntry(rustbus::signature::Base, rustbus::signature::Type),
}

#[derive(Debug)]
enum RustbusParamOrDictEntry<'a> {
    Rustbus(&'a rustbus::message::Param),
    RustbusBase(&'a rustbus::message::Base),
    DictEntry(&'a rustbus::message::Base, &'a rustbus::message::Param),
}

impl DBusMessageIter {
    fn append(&mut self, param: rustbus::message::Param) {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainAppendIter(msg) => {
                let msg = unsafe { &mut **msg };
                msg.msg.push_params(vec![param]);
            }
            MessageIterInternal::SubAppendIter(sub) => {
                sub.params.push(param);
            }
            _ => {
                // NO?!
                unimplemented!();
            }
        }
        self.counter += 1;
    }

    fn close(&mut self, parent: &mut DBusMessageIter) {
        if self.inner.is_null() {
            return;
        }
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainAppendIter(_msg) => {
                // nothing to do here
            }
            MessageIterInternal::SubAppendIter(sub) => match &sub.typ {
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
            _ => {
                // Weird but ok....
            }
        }
        std::mem::drop(unsafe { Box::from_raw(self.inner) });
    }

    fn len(&self) -> usize {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainAppendIter(_) => 0,
            MessageIterInternal::SubAppendIter(_) => 0,
            MessageIterInternal::MainIter(msg) => {
                let msg = unsafe { &**msg };
                msg.msg.params.len()
            }
            MessageIterInternal::ArrayIter(arr) => {
                let arr = unsafe { &**arr };
                arr.values.len()
            }
            MessageIterInternal::DictIter(dict) => {
                let dict = unsafe { &**dict };
                dict.map.len()
            }
            MessageIterInternal::VariantIter(_) => 1,
            MessageIterInternal::DictEntryIter(_, _) => 2,
            MessageIterInternal::StructIter(values) => {
                let values = unsafe { &**values };
                values.len()
            }
        }
    }

    fn has_next(&self) -> bool {
        let len = self.len();
        if len == 0 {
            false
        } else {
            self.counter < len - 1
        }
    }
    fn finished(&self) -> bool {
        let len = self.len();
        self.counter >= len
    }

    fn current(&self) -> Option<RustbusParamOrDictEntry<'_>> {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainAppendIter(_) => None,
            MessageIterInternal::SubAppendIter(_) => None,
            MessageIterInternal::MainIter(msg) => {
                let msg = unsafe { &**msg };
                if self.counter < msg.msg.params.len() {
                    Some(RustbusParamOrDictEntry::Rustbus(
                        &msg.msg.params[self.counter],
                    ))
                } else {
                    None
                }
            }
            MessageIterInternal::ArrayIter(arr) => {
                let arr = unsafe { &**arr };
                Some(RustbusParamOrDictEntry::Rustbus(&arr.values[self.counter]))
            }
            MessageIterInternal::DictIter(dict) => {
                let dict = unsafe { &**dict };
                let key = dict.map.keys().nth(self.counter).unwrap();
                let val = dict.map.get(key).unwrap();
                Some(RustbusParamOrDictEntry::DictEntry(key, val))
            }
            MessageIterInternal::VariantIter(var) => {
                let var = unsafe { &**var };
                Some(RustbusParamOrDictEntry::Rustbus(&var.value))
            }
            MessageIterInternal::DictEntryIter(key, val) => {
                if self.counter == 0 {
                    let key = unsafe { &**key };
                    Some(RustbusParamOrDictEntry::RustbusBase(key))
                } else if self.counter == 1 {
                    let val = unsafe { &**val };
                    Some(RustbusParamOrDictEntry::Rustbus(val))
                } else {
                    None
                }
            }
            MessageIterInternal::StructIter(values) => {
                let values = unsafe { &**values };
                Some(RustbusParamOrDictEntry::Rustbus(&values[self.counter]))
            }
        }
    }

    fn sig(&self) -> Option<Vec<RustbusTypeOrDictEntry>> {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainAppendIter(_) => None,
            MessageIterInternal::SubAppendIter(_) => None,
            MessageIterInternal::MainIter(msg) => {
                let msg = unsafe { &**msg };
                let mut sigs = Vec::new();
                for p in &msg.msg.params {
                    sigs.push(RustbusTypeOrDictEntry::Rustbus(p.sig()))
                }
                Some(sigs)
            }
            MessageIterInternal::ArrayIter(arr) => {
                let arr = unsafe { &**arr };
                Some(vec![RustbusTypeOrDictEntry::Rustbus(
                    rustbus::signature::Type::Container(rustbus::signature::Container::Array(
                        Box::new(arr.element_sig.clone()),
                    )),
                )])
            }
            MessageIterInternal::DictIter(dict) => {
                let dict = unsafe { &**dict };
                Some(vec![RustbusTypeOrDictEntry::Rustbus(
                    rustbus::signature::Type::Container(rustbus::signature::Container::Dict(
                        dict.key_sig.clone(),
                        Box::new(dict.value_sig.clone()),
                    )),
                )])
            }
            MessageIterInternal::VariantIter(_var) => Some(vec![RustbusTypeOrDictEntry::Rustbus(
                rustbus::signature::Type::Container(rustbus::signature::Container::Variant),
            )]),
            MessageIterInternal::DictEntryIter(key, val) => {
                let key = unsafe { &**key };
                let val = unsafe { &**val };
                if let rustbus::signature::Type::Base(key_sig) = key.sig() {
                    Some(vec![RustbusTypeOrDictEntry::DictEntry(key_sig, val.sig())])
                } else {
                    None
                }
            }
            MessageIterInternal::StructIter(values) => {
                let values = unsafe { &**values };
                let mut sigs = Vec::new();
                for p in values {
                    sigs.push(p.sig())
                }
                Some(vec![RustbusTypeOrDictEntry::Rustbus(
                    rustbus::signature::Type::Container(rustbus::signature::Container::Struct(
                        sigs,
                    )),
                )])
            }
        }
    }

    fn current_type(&self) -> Option<RustbusTypeOrDictEntry> {
        if self.finished() {
            return None;
        }

        let current = self.current();
        match current {
            Some(RustbusParamOrDictEntry::Rustbus(p)) => {
                Some(RustbusTypeOrDictEntry::Rustbus(p.sig()))
            }
            Some(RustbusParamOrDictEntry::RustbusBase(p)) => {
                Some(RustbusTypeOrDictEntry::Rustbus(p.sig()))
            }
            Some(RustbusParamOrDictEntry::DictEntry(k, v)) => {
                if let rustbus::signature::Type::Base(b) = k.sig() {
                    Some(RustbusTypeOrDictEntry::DictEntry(b, v.sig()))
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn current_element_type(&self) -> Option<RustbusTypeOrDictEntry> {
        if let Some(x) = self.current_type() {
            match x {
                RustbusTypeOrDictEntry::Rustbus(t) => match t {
                    rustbus::signature::Type::Container(rustbus::signature::Container::Array(
                        sig,
                    )) => Some(RustbusTypeOrDictEntry::Rustbus(sig.as_ref().clone())),
                    rustbus::signature::Type::Container(rustbus::signature::Container::Dict(
                        key_sig,
                        val_sig,
                    )) => Some(RustbusTypeOrDictEntry::DictEntry(
                        key_sig.clone(),
                        val_sig.as_ref().clone(),
                    )),
                    _ => None,
                },
                RustbusTypeOrDictEntry::DictEntry(_, _) => None,
            }
        } else {
            None
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_init(
    msg: *const crate::DBusMessage,
    args: *mut DBusMessageIter,
) -> u32 {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    *args = DBusMessageIter {
        inner: Box::into_raw(Box::new(MessageIterInternal::MainIter(msg))),
        counter: 0,
    };
    1
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_has_next(args: *mut DBusMessageIter) -> u32 {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    if args.has_next() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_next(args: *mut DBusMessageIter) -> u32 {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    args.counter += 1;
    if !args.finished() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_get_arg_type(args: *mut DBusMessageIter) -> libc::c_int {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    if let Some(t) = args.current_type() {
        match t {
            RustbusTypeOrDictEntry::Rustbus(t) => crate::rustbus_to_c_type(&t),
            RustbusTypeOrDictEntry::DictEntry(_, _) => crate::DBUS_TYPE_DICTENTRY,
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_get_element_type(args: *mut DBusMessageIter) -> libc::c_int {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    if let Some(t) = args.current_element_type() {
        match t {
            RustbusTypeOrDictEntry::Rustbus(t) => crate::rustbus_to_c_type(&t),
            RustbusTypeOrDictEntry::DictEntry(_, _) => crate::DBUS_TYPE_DICTENTRY,
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_recurse(
    parent: *mut DBusMessageIter,
    sub: *mut DBusMessageIter,
) {
    if parent.is_null() {
        return;
    }
    let parent = unsafe { &*parent };
    if sub.is_null() {
        return;
    }
    let sub = unsafe { &mut *sub };

    let current = parent.current();

    let iter = match current {
        Some(RustbusParamOrDictEntry::DictEntry(key, value)) => {
            MessageIterInternal::DictEntryIter(key, value)
        }
        Some(RustbusParamOrDictEntry::Rustbus(param)) => match param {
            rustbus::message::Param::Container(rustbus::message::Container::Array(arr)) => {
                MessageIterInternal::ArrayIter(arr)
            }
            rustbus::message::Param::Container(rustbus::message::Container::Dict(dict)) => {
                MessageIterInternal::DictIter(dict)
            }
            rustbus::message::Param::Container(rustbus::message::Container::Struct(values)) => {
                MessageIterInternal::StructIter(values)
            }
            rustbus::message::Param::Container(rustbus::message::Container::Variant(var)) => {
                MessageIterInternal::VariantIter(var.as_ref())
            }
            rustbus::message::Param::Base(_) => return,
        },
        Some(RustbusParamOrDictEntry::RustbusBase(_param)) => {
            return;
        }
        None => {
            return;
        }
    };

    *sub = DBusMessageIter {
        inner: Box::into_raw(Box::new(iter)),
        counter: 0,
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_get_signature(
    sub: *mut DBusMessageIter,
) -> *const libc::c_char {
    if sub.is_null() {
        return std::ptr::null();
    }
    let sub = unsafe { &mut *sub };
    let mut sigs_str = String::new();
    if let Some(sigs) = sub.sig() {
        for sig in sigs {
            match sig {
                RustbusTypeOrDictEntry::Rustbus(typ) => {
                    typ.to_str(&mut sigs_str);
                }
                RustbusTypeOrDictEntry::DictEntry(key, val) => {
                    sigs_str.push('{');
                    key.to_str(&mut sigs_str);
                    val.to_str(&mut sigs_str);
                    sigs_str.push('}');
                }
            }
        }
    } else {
        return std::ptr::null();
    }

    let cstr = std::ffi::CString::new(sigs_str.as_str()).unwrap();
    // needs to be freed somehow in dbus_free
    let ptr = cstr.into_raw();
    ptr
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_get_basic(
    sub: *mut DBusMessageIter,
    arg: *mut std::ffi::c_void,
) {
    if sub.is_null() {
        return;
    }
    let sub = unsafe { &mut *sub };

    if let Some(RustbusParamOrDictEntry::Rustbus(rustbus::message::Param::Base(base_param))) =
        sub.current()
    {
        crate::write_base_param(base_param, arg);
    }
    if let Some(RustbusParamOrDictEntry::RustbusBase(base_param)) = sub.current() {
        crate::write_base_param(base_param, arg);
    }
}
#[no_mangle]
pub extern "C" fn dbus_message_iter_get_element_count(sub: *mut DBusMessageIter) -> libc::c_int {
    if sub.is_null() {
        return 0;
    }
    let sub = unsafe { &mut *sub };

    sub.len() as libc::c_int
}
#[no_mangle]
pub extern "C" fn dbus_message_iter_get_fixed_array(
    _sub: *mut DBusMessageIter,
    _output: *mut std::ffi::c_void,
) {
    unimplemented!();
    // If this is really needed we need to somehow allocate memory since
    // we cant just point into our message struct.
    // One possibility would be to pass down a ref to the Message and
    // allocate it there. This would be suboptimal but would allow deallocation when the message gets
    // cleared
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_init_append(
    msg: *mut crate::DBusMessage,
    args: *mut DBusMessageIter,
) -> u32 {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    *args = DBusMessageIter {
        inner: Box::into_raw(Box::new(MessageIterInternal::MainAppendIter(msg))),
        counter: {
            let msg = unsafe { &*msg };
            msg.msg.params.len()
        },
    };
    1
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_append_basic(
    args: *mut DBusMessageIter,
    argtyp: libc::c_int,
    arg: *mut std::ffi::c_void,
) -> u32 {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };

    if let Some(param) = crate::param_from_parts(argtyp, arg) {
        args.append(param);
        1
    } else {
        0
    }
}

#[no_mangle]
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
        CStr::from_ptr(argsig)
    };

    let argsig = c_str.to_str().unwrap();
    let mut argsig = rustbus::signature::Type::parse_description(argsig).unwrap();
    let typ = match argtyp {
        crate::DBUS_TYPE_ARRAY => rustbus::signature::Container::Array(Box::new(argsig.remove(0))),
        crate::DBUS_TYPE_STRUCT => rustbus::signature::Container::Struct(argsig),
        crate::DBUS_TYPE_VARIANT => rustbus::signature::Container::Variant,
        _ => return,
    };

    *sub = DBusMessageIter {
        inner: Box::into_raw(Box::new(MessageIterInternal::SubAppendIter(
            SubAppendIter {
                params: Vec::new(),
                typ,
            },
        ))),
        counter: 0,
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_close_container(
    parent: *mut DBusMessageIter,
    sub: *mut DBusMessageIter,
) {
    let parent = unsafe { &mut *parent };
    let sub = unsafe { &mut *sub };
    sub.close(parent);
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_abandon_container(
    parent: *mut DBusMessageIter,
    sub: *mut DBusMessageIter,
) {
    // it dont think there is any harm in closing this properly
    dbus_message_iter_close_container(parent, sub);
}
#[no_mangle]
pub extern "C" fn dbus_message_iter_abandon_container_if_open(
    parent: *mut DBusMessageIter,
    sub: *mut DBusMessageIter,
) {
    // it dont think there is any harm in closing this properly
    // sub.close() checks if there there is a valid interator or not anyways
    dbus_message_iter_close_container(parent, sub);
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
pub extern "C" fn dbus_message_get_path(msg: *mut crate::DBusMessage) -> *const libc::c_char {
    if msg.is_null() {
        return std::ptr::null();
    }
    let msg = unsafe { &*msg };

    msg.msg
        .object
        .as_ref()
        .map(|p| unsafe { std::mem::transmute(p.as_ptr()) })
        .unwrap_or(std::ptr::null())
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

    if msg.msg.object == Some(path.to_owned()) {
        1
    } else {
        0
    }
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
    let msg = unsafe { &*msg };

    msg.msg
        .interface
        .as_ref()
        .map(|p| unsafe { std::mem::transmute(p.as_ptr()) })
        .unwrap_or(std::ptr::null())
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
    let msg = unsafe { &*msg };

    msg.msg
        .member
        .as_ref()
        .map(|p| unsafe { std::mem::transmute(p.as_ptr()) })
        .unwrap_or(std::ptr::null())
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
