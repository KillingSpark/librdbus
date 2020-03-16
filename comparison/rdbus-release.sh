#! /bin/sh

for i in {0..1000}
do
    LD_LIBRARY_PATH=../target/release/ /bin/dbus-send /some/object this.is.my.signal 2> /dev/null
done
