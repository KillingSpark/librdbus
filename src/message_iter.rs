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
    SubIter(*const rustbus::message::Container),
}

#[repr(C)]
pub struct DBusMessageIter {
    inner: *mut MessageIterInternal,
    counter: usize,
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

    fn has_next(&self) -> bool {
        let inner = unsafe { &mut *self.inner };
        match inner {
            MessageIterInternal::MainAppendIter(_) => false,
            MessageIterInternal::SubAppendIter(_) => false,
            MessageIterInternal::MainIter(msg) => {
                let msg = unsafe { &**msg };
                if self.counter < msg.msg.params.len() {
                    true
                } else {
                    false
                }
            }
            MessageIterInternal::SubIter(params) => {
                let params = unsafe { &**params };
                let len = match params {
                    rustbus::message::Container::Array(arr) => arr.values.len(),
                    rustbus::message::Container::Struct(values) => values.len(),
                    rustbus::message::Container::Dict(dict) => dict.map.len(),
                    rustbus::message::Container::Variant(_) => 1,
                };
                if self.counter < len {
                    true
                } else {
                    false
                }
            }
        }
    }
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
        counter: {
            let msg = unsafe { &*msg };
            msg.msg.params.len()
        },
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
    if args.has_next() {
        args.counter += 1;
        1
    } else {
        0
    }
}
