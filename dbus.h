#include <stdint.h>

typedef uint32_t dbus_uint32_t;

typedef struct DBusMessageIter {
  void *msg;
} DBusMessageIter;

typedef struct DBusError {
    char * message;
} DBusError;

typedef struct DBusMessage DBusMessage;

typedef enum {
  DBUS_BUS_SESSION, /**< The login session bus */
  DBUS_BUS_SYSTEM,  /**< The systemwide bus */
  DBUS_BUS_STARTER  /**< The bus that started us, if any */
} DBusBusType;

void *dbus_bus_get(int bus, void *err);
void dbus_connection_send_hello(void *, void *);
void dbus_connection_send(void *, void *, dbus_uint32_t *);
void dbus_error_init(DBusError *);
int dbus_error_is_set(DBusError *);
void *dbus_message_new_signal(char *, char *, char *);
void dbus_message_iter_init_append(DBusMessage *, DBusMessageIter *);
int dbus_message_iter_append_basic(DBusMessageIter *, int, void *);
void dbus_message_iter_open_container(DBusMessageIter *, int, char *,
                                      DBusMessageIter *);

#define DBUS_TYPE_BYTE ((int)'y')

#define DBUS_TYPE_BYTE_AS_STRING "y"

#define DBUS_TYPE_BOOLEAN ((int)'b')

#define DBUS_TYPE_BOOLEAN_AS_STRING "b"

#define DBUS_TYPE_INT16 ((int)'n')

#define DBUS_TYPE_INT16_AS_STRING "n"

#define DBUS_TYPE_UINT16 ((int)'q')

#define DBUS_TYPE_UINT16_AS_STRING "q"

#define DBUS_TYPE_INT32 ((int)'i')

#define DBUS_TYPE_INT32_AS_STRING "i"

#define DBUS_TYPE_UINT32 ((int)'u')

#define DBUS_TYPE_UINT32_AS_STRING "u"

#define DBUS_TYPE_INT64 ((int)'x')

#define DBUS_TYPE_INT64_AS_STRING "x"

#define DBUS_TYPE_UINT64 ((int)'t')

#define DBUS_TYPE_UINT64_AS_STRING "t"

#define DBUS_TYPE_DOUBLE ((int)'d')

#define DBUS_TYPE_DOUBLE_AS_STRING "d"

#define DBUS_TYPE_STRING ((int)'s')

#define DBUS_TYPE_STRING_AS_STRING "s"

#define DBUS_TYPE_OBJECT_PATH ((int)'o')

#define DBUS_TYPE_OBJECT_PATH_AS_STRING "o"

#define DBUS_TYPE_SIGNATURE ((int)'g')

#define DBUS_TYPE_SIGNATURE_AS_STRING "g"

#define DBUS_TYPE_UNIX_FD ((int)'h')

#define DBUS_TYPE_UNIX_FD_AS_STRING "h"

/* Compound types */
#define DBUS_TYPE_ARRAY ((int)'a')

#define DBUS_TYPE_ARRAY_AS_STRING "a"

#define DBUS_TYPE_VARIANT ((int)'v')

#define DBUS_TYPE_VARIANT_AS_STRING "v"

#define DBUS_TYPE_STRUCT ((int)'r')

#define DBUS_TYPE_STRUCT_AS_STRING "r"

#define DBUS_TYPE_DICT_ENTRY ((int)'e')

#define DBUS_TYPE_DICT_ENTRY_AS_STRING "e"