#! /bin/sh
# build librdbus
cargo build

# build the c example
mkdir -p target/c-example
gcc --std=c11 -o target/c-example/c-example test.c  \
    -I /usr/include/dbus-1.0/dbus                   \
    -I /usr/lib/dbus-1.0/include                    \
    -I /usr/include/dbus-1.0/                       \
    -L target/debug/                                \
    -llibrdbus
LD_LIBRARY_PATH=target/debug/ ./target/c-example/c-example