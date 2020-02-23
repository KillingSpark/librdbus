cargo build
gcc --std=c11 -o c-example test.c -L target/debug/ -llibrdbus
LD_LIBRARY_PATH=target/debug/ ./c-example