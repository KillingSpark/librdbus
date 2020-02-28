#include <stdint.h>

typedef uint32_t dbus_uint32_t;

typedef struct DBusMessageIter {
  void *msg;
  void *internal;
  uint64_t counter;
} DBusMessageIter;

typedef struct DBusError {
  int is_set;
  char *message;
  char padding[10];
} DBusError;

typedef struct DBusMessage DBusMessage;
typedef struct DBusConnection DBusConnection;

typedef enum {
  DBUS_BUS_SESSION, /**< The login session bus */
  DBUS_BUS_SYSTEM,  /**< The systemwide bus */
  DBUS_BUS_STARTER  /**< The bus that started us, if any */
} DBusBusType;

DBusConnection *dbus_bus_get(DBusBusType bus, DBusError *err);

void dbus_connection_close(DBusConnection *con);

uint32_t dbus_connection_send(DBusConnection *con, DBusMessage *msg,
                              uint32_t *serial);

uint32_t dbus_connection_send_hello(DBusConnection *con, uint32_t *serial);

void dbus_error_init(DBusError *err);

int dbus_error_is_set(DBusError *err);

uint32_t dbus_message_append_args(DBusMessage *msg, int typ1);

uint32_t dbus_message_append_args_valist(DBusMessage *msg, int typ1,
                                         void *_va_list);

uint32_t dbus_message_contains_unix_fds(DBusMessage *msg);

DBusMessage *dbus_message_copy(const DBusMessage *msg);

uint32_t dbus_message_get_args(DBusMessage *msg, int typ1);

uint32_t dbus_message_get_args_valist(DBusMessage *msg, int typ1,
                                      void *_va_list);

uint32_t dbus_message_get_reply_serial(const DBusMessage *msg);

const char *dbus_message_get_sender(const DBusMessage *msg);

uint32_t dbus_message_get_serial(const DBusMessage *msg);

int dbus_message_get_type(DBusMessage *msg);

uint32_t dbus_message_iter_append_basic(DBusMessageIter *args, int argtyp,
                                        void *arg);

void dbus_message_iter_close_container(DBusMessageIter *parent,
                                       DBusMessageIter *sub);

int dbus_message_iter_get_arg_type(DBusMessageIter *args);

int dbus_message_iter_get_element_type(DBusMessageIter *args);

uint32_t dbus_message_iter_has_next(DBusMessageIter *args);

uint32_t dbus_message_iter_init(const DBusMessage *msg, DBusMessageIter *args);

uint32_t dbus_message_iter_init_append(DBusMessage *msg, DBusMessageIter *args);

uint32_t dbus_message_iter_next(DBusMessageIter *args);

void dbus_message_iter_open_container(DBusMessageIter *parent, int argtyp,
                                      const char *argsig, DBusMessageIter *sub);

DBusMessage *dbus_message_new(int typ);

DBusMessage *dbus_message_new_error(const DBusMessage *call,
                                    const char *errname, const char *errmsg);

DBusMessage *dbus_message_new_error_printf(const DBusMessage *_call,
                                           const char *_errname,
                                           const char *_errmsg);

DBusMessage *dbus_message_new_method_call(const char *dest, const char *object,
                                          const char *interface,
                                          const char *member);

DBusMessage *dbus_message_new_method_return(const DBusMessage *call);

DBusMessage *dbus_message_new_signal(const char *object, const char *interface,
                                     const char *member);

DBusMessage *dbus_message_ref(DBusMessage *msg);

void dbus_message_iter_recurse(DBusMessageIter *, DBusMessageIter *);

char *dbus_message_iter_get_signature(DBusMessageIter*);

void dbus_message_iter_get_basic(DBusMessageIter*, void *);
uint32_t dbus_message_iter_get_element_count(DBusMessageIter *msg);

char *dbus_message_get_interface(DBusMessage *);
char *dbus_message_get_path(DBusMessage *);
char *dbus_message_get_member(DBusMessage *);

uint32_t dbus_message_set_reply_serial(DBusMessage *msg, uint32_t reply_serial);

void dbus_message_unref(DBusMessage *msg);

#define DBUS_TYPE_INVALID ((int)0)

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