use crate::dbus_bool;

#[repr(C)]
pub struct DBusError {
    pub error: Box<String>,
    pub name: Box<String>,
    pub is_set: bool,
}

#[no_mangle]
pub extern "C" fn dbus_error_init(err: *mut DBusError) {
    assert!(!err.is_null());
    let err = unsafe { &mut *err };
    let mut new_err = DBusError {
        error: Box::new(String::new()),
        name: Box::new(String::new()),
        is_set: false,
    };
    std::mem::swap(err, &mut new_err);
    std::mem::forget(new_err);
}
#[no_mangle]
pub extern "C" fn dbus_error_free(err: *mut DBusError) {
    let err = unsafe { &mut *err };
    err.error = Box::new(String::new());
    err.is_set = false;
}

#[no_mangle]
pub extern "C" fn dbus_error_is_set(err: *mut DBusError) -> libc::c_int {
    if err.is_null() {
        return 0;
    }

    let err: &mut DBusError = unsafe { &mut *err };
    if err.is_set {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn dbus_error_has_name(err: *mut DBusError, name: *const libc::c_char) -> u32 {
    if err.is_null() {
        return 0;
    }
    let err = unsafe { &mut *err };

    let c_str = unsafe {
        assert!(!name.is_null());
        std::ffi::CStr::from_ptr(name)
    };
    let name = c_str.to_str().unwrap();

    dbus_bool(err.name.as_ref().eq(name))
}
