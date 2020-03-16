#! /bin/sh

#prepare with newest version
cd ..
cargo build > /dev/null 2> /dev/null
cp target/debug/liblibrdbus.so target/debug/libdbus-1.so.3

cargo build --release > /dev/null 2> /dev/null
cp target/release/liblibrdbus.so target/release/libdbus-1.so.3
cd comparison


time ./rdbus.sh
time ./rdbus-release.sh
time ./dbus.sh