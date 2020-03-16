#! /bin/sh

for i in {0..1000}
do
    /bin/dbus-send /some/object this.is.my.signal
done
