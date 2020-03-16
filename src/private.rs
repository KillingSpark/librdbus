#[no_mangle]
pub extern "C" fn _dbus_get_real_time(secs: *mut libc::c_long, micro_secs: *mut libc::c_long) {
    let time = std::time::SystemTime::now();
    let dur = time.duration_since(std::time::UNIX_EPOCH).unwrap();

    if !secs.is_null() {
        unsafe { *secs = dur.as_secs() as libc::c_long };
    }
    if !micro_secs.is_null() {
        unsafe { *micro_secs = dur.subsec_micros() as libc::c_long };
    }
}
