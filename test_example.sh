#! /bin/sh
# build librdbus
cargo build

# build the c example
mkdir -p target/c-example
gcc --std=c11 -o target/c-example/c-example test.c -L target/debug/ -llibrdbus
LD_LIBRARY_PATH=target/debug/ ./target/c-example/c-example