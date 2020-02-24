#include "dbus.h"
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

char *text = "THIS IS A STRING FROM C";

void make_msg(DBusMessageIter *iter) {

  DBusMessageIter sub;
  dbus_message_iter_open_container(iter, DBUS_TYPE_ARRAY, "s", &sub);
  for (int i = 0; i < 2; i++) {
    if (!dbus_message_iter_append_basic(&sub, DBUS_TYPE_STRING, &text)) {
      fprintf(stderr, "Out Of Memory!\n");
      exit(1);
    }
  }
  dbus_message_iter_close_container(iter, &sub);

  int16_t val = 256;
  dbus_message_iter_append_basic(iter, DBUS_TYPE_INT16, &val);
  int16_t val2 = -128;
  dbus_message_iter_append_basic(iter, DBUS_TYPE_INT16, &val2);
}

void get_and_print_base(DBusMessageIter *iter, int sig) {
  switch (sig) {
  case DBUS_TYPE_STRING: {
    char *string;
    dbus_message_iter_get_basic(iter, &string);
    printf("String: %s\n", string);
    break;
  }
  case DBUS_TYPE_INT16: {
    int16_t val;
    dbus_message_iter_get_basic(iter, &val);
    printf("Int16: %d\n", val);
    break;
  }
  }
}

void print_iter(DBusMessageIter *iter) {
  printf("Start printing iter\n");
  char *sig = dbus_message_iter_get_signature(iter);
  if (sig)
    printf("Iter signature: %s\n", sig);
  int current_type = 0;
  while ((current_type = dbus_message_iter_get_arg_type(iter)) !=
         DBUS_TYPE_INVALID) {
    printf("TYPE: %c\n", current_type);

    switch (current_type) {
    default: {
      get_and_print_base(iter, current_type);
      break;
    }
    case DBUS_TYPE_ARRAY: {
      int element_type = dbus_message_iter_get_element_type(iter);
      if (element_type) {
        printf("ELEMENT_TYPE: %c\n", element_type);
        DBusMessageIter sub_iter;
        dbus_message_iter_recurse(iter, &sub_iter);
        print_iter(&sub_iter);
      } else {
        printf("Couldnt get type for array elements!");
        exit(1);
      }
      break;
    }
    }

    dbus_message_iter_next(iter);
  }
  printf("End printing iter\n");
}

void print_msg(DBusMessage *msg) {
  DBusMessageIter iter;
  dbus_message_iter_init(msg, &iter);

  printf("Start printing message\n");
  print_iter(&iter);
  printf("End printing message\n");
}

int main(void) {
  DBusError error;
  dbus_error_init(&error);

  void *con = dbus_bus_get(DBUS_BUS_SESSION, &error);

  uint32_t serial = 0;
  dbus_connection_send_hello(con, &serial);
  DBusMessage *sig = dbus_message_new_signal(
      "/test/signal/Object", // object name of the signal
      "test.signal.Type",    // interface name of the signal
      "Test");               // name of the signal

  DBusMessageIter args;
  dbus_message_iter_init_append(sig, &args);
  make_msg(&args);

  print_msg(sig);

  dbus_connection_send(con, sig, &serial);
}