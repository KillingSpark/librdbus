use rustbus::params;
use rustbus::signature;
use std::ffi::CStr;

pub struct SubAppendIter<'a> {
    params: Vec<params::Param<'a, 'a>>,
    typ: rustbus::signature::Container,
}

enum MessageIterInternal<'a> {
    // pushes contents into message
    MainAppendIter(*mut crate::DBusMessage<'a>),
    // pushes contents into parent when closed
    SubAppendIter(SubAppendIter<'a>),
    MainIter(*const crate::DBusMessage<'a>),
    StructIter(*const [params::Param<'a, 'a>]),
    DictIter(
        *const params::DictMap<'a, 'a>,
        *const signature::Base,
        *const signature::Type,
    ),
    ArrayIter(*const [params::Param<'a, 'a>], *const signature::Type),
    VariantIter(*const params::Variant<'a, 'a>),
    DictEntryIter(*const params::Base<'a>, *const params::Param<'a, 'a>),
}

#[repr(C)]
pub struct DBusMessageIter<'a> {
    inner: *mut MessageIterInternal<'a>,
    counter: usize,
    msg: *mut crate::DBusMessage<'a>,
}

#[derive(Debug)]
enum RustbusTypeOrDictEntry {
    Rustbus(rustbus::signature::Type),
    DictEntry(rustbus::signature::Base, rustbus::signature::Type),
}

#[derive(Debug)]
enum RustbusParamOrDictEntry<'a> {
    Rustbus(&'a params::Param<'a, 'a>),
    RustbusBase(&'a params::Base<'a>),
    DictEntry(&'a params::Base<'a>, &'a params::Param<'a, 'a>),
}

impl<'a> DBusMessageIter<'a> {
    fn append(&mut self, param: params::Param<'a, 'a>) {
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

    fn close(&mut self, parent: &mut DBusMessageIter<'a>) {
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
                    params::Container::Array(params::Array {
                        element_sig: sig.as_ref().clone(),
                        values: sub.params.clone(),
                    })
                    .into(),
                ),
                rustbus::signature::Container::Dict(_, _) => unimplemented!(),
                rustbus::signature::Container::Variant => parent.append(
                    params::Container::Variant(Box::new(params::Variant {
                        sig: sub.params[0].sig(),
                        value: sub.params[0].clone(),
                    }))
                    .into(),
                ),
                rustbus::signature::Container::Struct(_sigs) => {
                    parent.append(params::Container::Struct(sub.params.clone()).into())
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
            MessageIterInternal::ArrayIter(arr, _) => {
                let arr = unsafe { &**arr };
                arr.len()
            }
            MessageIterInternal::DictIter(dict, _, _) => {
                let dict = unsafe { &**dict };
                dict.len()
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
            MessageIterInternal::ArrayIter(arr, _) => {
                let arr = unsafe { &**arr };
                Some(RustbusParamOrDictEntry::Rustbus(&arr[self.counter]))
            }
            MessageIterInternal::DictIter(dict, _, _) => {
                let dict = unsafe { &**dict };
                let key = dict.keys().nth(self.counter).unwrap();
                let val = dict.get(key).unwrap();
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
            MessageIterInternal::ArrayIter(_, sig) => {
                let sig = unsafe { &**sig };
                Some(vec![RustbusTypeOrDictEntry::Rustbus(
                    rustbus::signature::Type::Container(rustbus::signature::Container::Array(
                        Box::new(sig.clone()),
                    )),
                )])
            }
            MessageIterInternal::DictIter(_, k, v) => {
                let k = unsafe { &**k };
                let v = unsafe { &**v };
                Some(vec![RustbusTypeOrDictEntry::Rustbus(
                    rustbus::signature::Type::Container(rustbus::signature::Container::Dict(
                        *k,
                        Box::new(v.clone()),
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
pub extern "C" fn dbus_message_iter_init<'a>(
    msg: *mut crate::DBusMessage<'a>,
    args: *mut DBusMessageIter<'a>,
) -> u32 {
    if args.is_null() {
        return 0;
    }
    let args = unsafe { &mut *args };
    *args = DBusMessageIter {
        inner: Box::into_raw(Box::new(MessageIterInternal::MainIter(msg))),
        counter: 0,
        msg,
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
pub extern "C" fn dbus_message_iter_recurse<'a>(
    parent: *mut DBusMessageIter<'a>,
    sub: *mut DBusMessageIter<'a>,
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
            params::Param::Container(params::Container::Array(arr)) => {
                MessageIterInternal::ArrayIter(arr.values.as_slice(), &arr.element_sig)
            }
            params::Param::Container(params::Container::Dict(dict)) => {
                MessageIterInternal::DictIter(&dict.map, &dict.key_sig, &dict.value_sig)
            }
            params::Param::Container(params::Container::Struct(values)) => {
                MessageIterInternal::StructIter(values.as_slice())
            }
            params::Param::Container(params::Container::Variant(var)) => {
                MessageIterInternal::VariantIter(var.as_ref())
            }
            params::Param::Container(params::Container::ArrayRef(arr)) => {
                MessageIterInternal::ArrayIter(arr.values, &arr.element_sig)
            }
            params::Param::Container(params::Container::DictRef(dict)) => {
                MessageIterInternal::DictIter(dict.map, &dict.key_sig, &dict.value_sig)
            }
            params::Param::Container(params::Container::StructRef(values)) => {
                MessageIterInternal::StructIter(*values)
            }
            params::Param::Base(_) => return,
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
        msg: parent.msg,
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
    let string_arena = &mut unsafe { &mut *(&mut *sub).msg }.string_arena;
    let sub = unsafe { &mut *sub };

    if let Some(RustbusParamOrDictEntry::Rustbus(params::Param::Base(base_param))) = sub.current() {
        crate::write_base_param(base_param, string_arena, arg);
    }
    if let Some(RustbusParamOrDictEntry::RustbusBase(base_param)) = sub.current() {
        crate::write_base_param(base_param, string_arena, arg);
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
pub extern "C" fn dbus_message_iter_init_append<'a>(
    msg: *mut crate::DBusMessage<'a>,
    args: *mut DBusMessageIter<'a>,
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
        msg,
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
pub extern "C" fn dbus_message_iter_open_container<'a>(
    parent: *mut DBusMessageIter<'a>,
    argtyp: libc::c_int,
    argsig: *const libc::c_char,
    sub: *mut DBusMessageIter<'a>,
) {
    if parent.is_null() {
        return;
    }
    let parent = unsafe { &mut *parent };
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
        msg: parent.msg,
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_close_container<'a>(
    parent: *mut DBusMessageIter<'a>,
    sub: *mut DBusMessageIter<'a>,
) {
    let parent = unsafe { &mut *parent };
    let sub = unsafe { &mut *sub };
    sub.close(parent);
}

#[no_mangle]
pub extern "C" fn dbus_message_iter_abandon_container<'a>(
    parent: *mut DBusMessageIter<'a>,
    sub: *mut DBusMessageIter<'a>,
) {
    // it dont think there is any harm in closing this properly
    dbus_message_iter_close_container(parent, sub);
}
#[no_mangle]
pub extern "C" fn dbus_message_iter_abandon_container_if_open<'a>(
    parent: *mut DBusMessageIter<'a>,
    sub: *mut DBusMessageIter<'a>,
) {
    // it dont think there is any harm in closing this properly
    // sub.close() checks if there there is a valid interator or not anyways
    dbus_message_iter_close_container(parent, sub);
}
