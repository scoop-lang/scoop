#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gio/gio.h>
#pragma clang diagnostic pop
#include <stdint.h>
#include <string.h>

typedef struct {
  GDBusConnection *connection;
  guint filter;
  gint references;
  GMutex mutex;
  int32_t values[6];
} PortalProbe;

static void release_probe(gpointer data) {
  PortalProbe *probe = data;
  if (g_atomic_int_dec_and_test(&probe->references)) {
    g_mutex_clear(&probe->mutex);
    g_free(probe);
  }
}

/* Observe the real GTK connection; do not implement or replace the portal. */
static GDBusMessage *observe(GDBusConnection *connection, GDBusMessage *message,
                             gboolean incoming, gpointer data) {
  PortalProbe *probe = data;
  const char *interface = g_dbus_message_get_interface(message);
  const char *method = g_dbus_message_get_member(message);
  GVariant *body = g_dbus_message_get_body(message);
  g_mutex_lock(&probe->mutex);
  if (!incoming && interface && method &&
      !strcmp(interface, "org.freedesktop.portal.FileChooser") &&
      (!strcmp(method, "OpenFile") || !strcmp(method, "SaveFile")) && body &&
      g_variant_is_of_type(body, G_VARIANT_TYPE("(ssa{sv})"))) {
    const char *parent, *title;
    GVariant *options;
    gboolean multiple = FALSE, directory = FALSE;
    g_variant_get(body, "(&s&s@a{sv})", &parent, &title, &options);
    g_variant_lookup(options, "multiple", "b", &multiple);
    g_variant_lookup(options, "directory", "b", &directory);
    GVariant *filters = g_variant_lookup_value(options, "filters", NULL);
    ++probe->values[0];
    probe->values[3] = !strcmp(method, "SaveFile") ? 2
                       : directory                 ? 3
                       : multiple                  ? 1
                                                   : 0;
    probe->values[4] = filters && g_variant_n_children(filters) > 0;
    probe->values[5] = !strcmp(title, "Scoop portal probe");
    g_clear_pointer(&filters, g_variant_unref);
    g_variant_unref(options);
  } else if (incoming &&
             g_dbus_message_get_message_type(message) ==
                 G_DBUS_MESSAGE_TYPE_METHOD_RETURN &&
             body && g_variant_is_of_type(body, G_VARIANT_TYPE("(o)"))) {
    const char *path;
    g_variant_get(body, "(&o)", &path);
    if (g_str_has_prefix(path, "/org/freedesktop/portal/desktop/request/"))
      ++probe->values[1];
  } else if (!incoming && interface && method &&
             !strcmp(interface, "org.freedesktop.portal.Request") &&
             !strcmp(method, "Close")) {
    ++probe->values[2];
  }
  g_mutex_unlock(&probe->mutex);
  return message;
}

PortalProbe *scoop_test_portal_create(void) {
  GDBusConnection *connection = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, NULL);
  if (!connection)
    return NULL;
  PortalProbe *probe = g_new0(PortalProbe, 1);
  probe->connection = connection;
  probe->references = 2;
  g_mutex_init(&probe->mutex);
  probe->filter =
      g_dbus_connection_add_filter(connection, observe, probe, release_probe);
  return probe;
}

int32_t scoop_test_portal_value(PortalProbe *probe, int32_t field) {
  g_mutex_lock(&probe->mutex);
  int32_t result = probe->values[field];
  g_mutex_unlock(&probe->mutex);
  return result;
}

void scoop_test_portal_release(PortalProbe *probe) {
  g_dbus_connection_remove_filter(probe->connection, probe->filter);
  g_object_unref(probe->connection);
  release_probe(probe);
}
