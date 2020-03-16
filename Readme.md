# What is this?
librdbus is a toy project that tries to reimplement the widely used libdbus in rust. This uses [rustbus](https://github.com/KillingSpark/rustbus) internally.

The goal is to learn about rust ffi. And have a cool project to annoy C-Devs with.
I only read the [doc](https://dbus.freedesktop.org/doc/api/html/) and no code while doing this, so there might be some inconsistencies there.

## State
Currently librdbus can be used as a dropin for programs using a small subset of the libdbus API. Look at the dbus-send script to see how that can be done.
This can succesfully replace libdbus for dbus-send and dbus-monitor.

dbus-monitor sometimes calls unimplemented parts of the interator API. Everything else seems to work.

Some early tests indicate that librdbus has the potential to outperform libdbus (see the comparison scripts).

