use crate::connection::*;
use crate::*;
use crate::error::*;

#[no_mangle]
pub extern "C" fn dbus_bus_register<'a>(con: *mut DBusConnection<'a>, err: *mut DBusError) -> u32 {
    if con.is_null() {
        return 0;
    }
    let con = unsafe { &mut *con };
    let _serial = match con
        .con
        .send_message(&mut rustbus::standard_messages::hello(), None)
    {
        Ok(sent_serial) => sent_serial,
        Err(_e) => return dbus_bool(false),
    };

    // TODO check the message for serial/types etc

    let resp = con.con.get_next_message(None).unwrap();
    let unique_name = resp.params[0].as_str().unwrap();
    con.unique_name = Some(std::ffi::CString::new(unique_name).unwrap());

    dbus_bool(true)
}
