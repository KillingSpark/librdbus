use crate::dbus_bool;
use crate::DBusFreeFunction;

struct Slot {
    id: i32,
    ref_count: i64,
}

// TODO lock this with a mutex?
static mut USED_SLOTS: Vec<Slot> = Vec::new();

fn find_new_slot_id(slots: &Vec<Slot>) -> Option<i32> {
    for id in 0..i32::max_value() {
        let mut found = false;
        for s in slots {
            if s.id == id {
                found = true;
                break;
            }
        }
        if !found {
            return Some(id);
        }
    }
    None
}

fn insert_new_slot(id: i32, slots: &mut Vec<Slot>) {
    slots.push(Slot { id, ref_count: 1 })
}
fn ref_slot(id: i32, slots: &mut Vec<Slot>) {
    for s in slots {
        if s.id == id {
            s.ref_count += 1;
        }
    }
}
fn unref_slot(id: i32, slots: &mut Vec<Slot>) {
    for s in slots {
        if s.id == id {
            s.ref_count -= 1;
            if s.ref_count <= 0 {
                s.ref_count = -1;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn dbus_message_allocate_data_slot(slotp: *mut i32) -> u32 {
    if slotp.is_null() {
        return dbus_bool(false);
    }
    let slotp = unsafe { &mut *slotp };
    if *slotp == -1 {
        if let Some(new_id) = find_new_slot_id(unsafe { &USED_SLOTS }) {
            *slotp = new_id;
            insert_new_slot(new_id, unsafe { &mut USED_SLOTS });
        } else {
            return dbus_bool(false);
        }
    } else {
        ref_slot(*slotp, unsafe { &mut USED_SLOTS })
    }
    dbus_bool(true)
}

#[no_mangle]
pub extern "C" fn dbus_message_free_data_slot(slotp: *mut i32) {
    if slotp.is_null() {
        return;
    }
    let slotp = unsafe { *slotp };

    unref_slot(slotp, unsafe { &mut USED_SLOTS })
}

#[derive(Clone, Debug)]
pub struct AppData {
    pub slot: i32,
    pub data: *mut std::ffi::c_void,
    pub free: Option<DBusFreeFunction>,
    freed: bool,
}

impl Drop for AppData {
    fn drop(&mut self) {
        if !self.freed {
            self.freed = true;
            if let Some(free_fn) = self.free {
                free_fn(self.data);
            }
        }
    }
}

fn replace_data(old: &mut AppData, new: AppData) {
    if let Some(free_fn) = old.free {
        free_fn(old.data);
        old.freed = true;
    }
    *old = new;
}

#[no_mangle]
pub extern "C" fn dbus_message_set_data(
    msg: *mut crate::DBusMessage,
    slot: i32,
    data: *mut std::ffi::c_void,
    free_fn: *const DBusFreeFunction,
) -> u32 {
    if msg.is_null() {
        return dbus_bool(false);
    }
    let msg = unsafe { &mut *msg };

    let free = if free_fn.is_null() {
        None
    } else {
        let free = unsafe { *free_fn };
        Some(free)
    };

    let app_data = AppData {
        slot,
        data,
        free,
        freed: false,
    };

    for a in &mut msg.app_data {
        if a.slot == slot {
            replace_data(a, app_data);
            return dbus_bool(true);
        }
    }

    // only get here if not replaced
    msg.app_data.push(app_data);

    dbus_bool(true)
}

#[no_mangle]
pub extern "C" fn dbus_message_get_data(
    msg: *mut crate::DBusMessage,
    slot: i32,
) -> *mut std::ffi::c_void {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &mut *msg };

    for a in &mut msg.app_data {
        if a.slot == slot {
            return a.data;
        }
    }
    std::ptr::null_mut()
}
