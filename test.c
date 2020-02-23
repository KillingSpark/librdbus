#include "dbus.h"
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

void make_msg(DBusMessageIter *iter) {
  DBusMessageIter sub;
  dbus_message_iter_open_container(iter, DBUS_TYPE_ARRAY, "s", &sub);
  for (int i = 0; i < 100; i++) {
    char *text = "THIS IS A STRING FROM C";
    if (!dbus_message_iter_append_basic(&sub, DBUS_TYPE_STRING, &text)) {
      fprintf(stderr, "Out Of Memory!\n");
      exit(1);
    }
  }
  dbus_message_iter_close_container(iter, &sub);
}

int main(void) {
  DBusError error;
  dbus_error_init(&error);

  void *con = dbus_bus_get(DBUS_BUS_SESSION, &error);
  dbus_connection_send_hello(con, &error);
  DBusMessage *sig = dbus_message_new_signal(
      "/test/signal/Object", // object name of the signal
      "test.signal.Type",    // interface name of the signal
      "Test");               // name of the signal

  DBusMessageIter args;
  dbus_message_iter_init_append(sig, &args);
  make_msg(&args);

  uint32_t serial = 0;
  dbus_connection_send(con, sig, &serial);
}