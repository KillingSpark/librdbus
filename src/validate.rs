use crate::*;

#[no_mangle]
pub extern "C" fn dbus_validate_bus_name(s: *const libc::c_char, _err: *mut DBusError) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };
    let s = c_str.to_str().unwrap();

    // TODO set error
    dbus_bool(rustbus::params::validate_busname(s).is_ok())
}

#[no_mangle]
pub extern "C" fn dbus_validate_bus_path(s: *const libc::c_char, _err: *mut DBusError) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };
    let s = c_str.to_str().unwrap();

    // TODO set error
    dbus_bool(rustbus::params::validate_object_path(s).is_ok())
}
#[no_mangle]
pub extern "C" fn dbus_validate_interface(s: *const libc::c_char, _err: *mut DBusError) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };
    let s = c_str.to_str().unwrap();

    // TODO set error
    dbus_bool(rustbus::params::validate_interface(s).is_ok())
}
#[no_mangle]
pub extern "C" fn dbus_validate_member(s: *const libc::c_char, _err: *mut DBusError) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };
    let s = c_str.to_str().unwrap();

    // TODO set error
    dbus_bool(rustbus::params::validate_membername(s).is_ok())
}
#[no_mangle]
pub extern "C" fn dbus_validate_error_name(s: *const libc::c_char, _err: *mut DBusError) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };
    let s = c_str.to_str().unwrap();

    // TODO set error
    dbus_bool(rustbus::params::validate_errorname(s).is_ok())
}
#[no_mangle]
pub extern "C" fn dbus_validate_utf8(s: *const libc::c_char, _err: *mut DBusError) -> u32 {
    let c_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s)
    };

    // TODO set error
    dbus_bool(c_str.to_str().is_ok())
}
