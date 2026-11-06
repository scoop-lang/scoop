#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gio/gio.h>
#pragma clang diagnostic pop
#include <stdint.h>
#include <string.h>

#define WATCHER "org.kde.StatusNotifierWatcher"
#define WATCHER_PATH "/StatusNotifierWatcher"
#define ITEM "org.kde.StatusNotifierItem"
#define MENU "com.canonical.dbusmenu"

typedef struct {
  GDBusConnection *bus;
  GCancellable *cancel;
  guint registration, pending, registrations;
  GDBusNodeInfo *interface;
  char *service, *path, *menu_path;
  int stage, status, command, retired;
  gboolean closing;
} TestHost;

static const char xml[] =
    "<node><interface name='" WATCHER "'>"
    "<method name='RegisterStatusNotifierItem'><arg type='s' "
    "direction='in'/></method>"
    "<property name='IsStatusNotifierHostRegistered' type='b' access='read'/>"
    "<property name='ProtocolVersion' type='i' access='read'/>"
    "<property name='RegisteredStatusNotifierItems' type='as' access='read'/>"
    "<signal name='StatusNotifierHostRegistered'/><signal "
    "name='StatusNotifierHostUnregistered'/>"
    "</interface></node>";

static void watcher_method(GDBusConnection *bus, const gchar *sender,
                           const gchar *path, const gchar *interface,
                           const gchar *name, GVariant *parameters,
                           GDBusMethodInvocation *invocation, gpointer data) {
  TestHost *host = data;
  const char *item;
  g_variant_get(parameters, "(&s)", &item);
  if (*item != '/') {
    host->status = -1;
    g_dbus_method_invocation_return_error(invocation, G_DBUS_ERROR,
                                          G_DBUS_ERROR_INVALID_ARGS,
                                          "Expected an object path");
    return;
  }
  g_free(host->path);
  g_free(host->service);
  host->path = g_strdup(item);
  host->service = g_strdup(sender);
  ++host->registrations;
  g_dbus_method_invocation_return_value(invocation, NULL);
}

static GVariant *watcher_property(GDBusConnection *bus, const gchar *sender,
                                  const gchar *path, const gchar *interface,
                                  const gchar *name, GError **error,
                                  gpointer data) {
  TestHost *host = data;
  if (!strcmp(name, "IsStatusNotifierHostRegistered"))
    return g_variant_new_boolean(TRUE);
  if (!strcmp(name, "ProtocolVersion"))
    return g_variant_new_int32(0);
  GVariantBuilder items;
  g_variant_builder_init(&items, G_VARIANT_TYPE("as"));
  if (host->service)
    g_variant_builder_add(&items, "s", host->service);
  return g_variant_builder_end(&items);
}

static const GDBusInterfaceVTable watcher_vtable = {
    .method_call = watcher_method, .get_property = watcher_property};

int32_t scoop_test_linux_tray_cycle(void *pointer, int32_t present) {
  TestHost *host = pointer;
  GError *error = NULL;
  GVariant *reply = g_dbus_connection_call_sync(
      host->bus, "org.freedesktop.DBus", "/org/freedesktop/DBus",
      "org.freedesktop.DBus", present ? "RequestName" : "ReleaseName",
      present ? g_variant_new("(su)", WATCHER, 4u)
              : g_variant_new("(s)", WATCHER),
      G_VARIANT_TYPE("(u)"), G_DBUS_CALL_FLAGS_NONE, 3000, NULL, &error);
  guint result = 0;
  if (reply) {
    g_variant_get(reply, "(u)", &result);
    g_variant_unref(reply);
  }
  g_clear_error(&error);
  return result == 1;
}

static void call(TestHost *host, const char *path, const char *interface,
                 const char *method, GVariant *parameters,
                 const GVariantType *reply, GAsyncReadyCallback callback) {
  ++host->pending;
  g_dbus_connection_call(host->bus, host->service, path, interface, method,
                         parameters, reply, G_DBUS_CALL_FLAGS_NONE, 3000,
                         host->cancel, callback, host);
}

static GVariant *finish(GObject *source, GAsyncResult *result, TestHost *host,
                        int error_code) {
  GError *error = NULL;
  GVariant *reply =
      g_dbus_connection_call_finish(G_DBUS_CONNECTION(source), result, &error);
  --host->pending;
  if (error && error_code && !host->closing) {
    g_printerr("tray protocol: %s\n", error->message);
    host->status = error_code;
  }
  g_clear_error(&error);
  return reply;
}

static int find_item(GVariant *tree, const char *label, GVariant **properties) {
  gint id;
  GVariant *values, *children;
  g_variant_get(tree, "(i@a{sv}@av)", &id, &values, &children);
  const char *caption = NULL;
  int found = 0;
  if (g_variant_lookup(values, "label", "&s", &caption) &&
      !strcmp(caption, label)) {
    found = id;
    if (properties)
      *properties = g_variant_ref(values);
  } else {
    GVariantIter iterator;
    GVariant *child;
    g_variant_iter_init(&iterator, children);
    while (!found && g_variant_iter_next(&iterator, "v", &child)) {
      found = find_item(child, label, properties);
      g_variant_unref(child);
    }
  }
  g_variant_unref(values);
  g_variant_unref(children);
  return found;
}

static void activated(GObject *source, GAsyncResult *result, gpointer data) {
  TestHost *host = data;
  GVariant *reply = finish(source, result, host, -25);
  if (reply) {
    host->status = host->stage;
    g_variant_unref(reply);
  }
}

static void retired_clicked(GObject *source, GAsyncResult *result,
                            gpointer data) {
  TestHost *host = data;
  GVariant *reply = finish(source, result, host, 0);
  g_clear_pointer(&reply, g_variant_unref);
  if (!host->closing)
    call(host, host->menu_path, MENU, "Event",
         g_variant_new("(isvu)", host->command, "clicked",
                       g_variant_new_int32(0), 0u),
         G_VARIANT_TYPE_UNIT, activated);
}

static void layout(GObject *source, GAsyncResult *result, gpointer data) {
  TestHost *host = data;
  GVariant *reply = finish(source, result, host, -10);
  if (!reply || host->closing) {
    g_clear_pointer(&reply, g_variant_unref);
    return;
  }
  guint revision;
  GVariant *tree, *properties = NULL;
  g_variant_get(reply, "(u@(ia{sv}av))", &revision, &tree);
  int command =
      find_item(tree, host->stage == 1 ? "Execute" : "Renamed", &properties);
  if (!command || (host->stage != 1 && command != host->command))
    host->status = -11;
  if (host->stage == 1 && properties) {
    host->command = command;
    host->retired = find_item(tree, "Retire", NULL);
    GVariant *icon =
        g_variant_lookup_value(properties, "icon-data", G_VARIANT_TYPE("ay"));
    GVariant *shortcut =
        g_variant_lookup_value(properties, "shortcut", G_VARIANT_TYPE("aas"));
    if (!host->retired || !icon || g_variant_n_children(icon) < 8 ||
        !shortcut || !g_variant_n_children(shortcut))
      host->status = -12;
    g_clear_pointer(&icon, g_variant_unref);
    g_clear_pointer(&shortcut, g_variant_unref);
  } else if (find_item(tree, "Retire", NULL)) {
    host->status = -13;
  }
  g_clear_pointer(&properties, g_variant_unref);
  if (!find_item(tree, "Checked", &properties)) {
    host->status = -14;
  } else {
    gint state = -1;
    const char *type = NULL;
    g_variant_lookup(properties, "toggle-type", "&s", &type);
    g_variant_lookup(properties, "toggle-state", "i", &state);
    if (!type || strcmp(type, "checkmark") || state != (host->stage != 1))
      host->status = -15;
  }
  g_clear_pointer(&properties, g_variant_unref);
  if (!find_item(tree, "Radio", NULL) || !find_item(tree, "Nested", NULL))
    host->status = -16;
  g_variant_unref(tree);
  g_variant_unref(reply);
  if (host->status < 0)
    return;
  if (host->stage == 1) {
    host->status = 1;
  } else if (host->stage == 2) {
    call(host, host->menu_path, MENU, "Event",
         g_variant_new("(isvu)", host->retired, "clicked",
                       g_variant_new_int32(0), 0u),
         G_VARIANT_TYPE_UNIT, retired_clicked);
  } else {
    if (host->registrations != 2) {
      host->status = -17;
      return;
    }
    call(host, host->path, ITEM, "Activate", g_variant_new("(ii)", -75, 22),
         G_VARIANT_TYPE_UNIT, activated);
  }
}

static void snapshot(GObject *source, GAsyncResult *result, gpointer data) {
  TestHost *host = data;
  GVariant *reply = finish(source, result, host, -2);
  if (!reply || host->closing) {
    g_clear_pointer(&reply, g_variant_unref);
    return;
  }
  GVariant *properties;
  g_variant_get(reply, "(@a{sv})", &properties);
  const char *title = NULL, *menu = NULL;
  g_variant_lookup(properties, "Title", "&s", &title);
  g_variant_lookup(properties, "Menu", "&o", &menu);
  if (!title || strcmp(title, host->stage == 1 ? "Before" : "After") || !menu ||
      !strcmp(menu, "/"))
    host->status = -3;
  else {
    g_free(host->menu_path);
    host->menu_path = g_strdup(menu);
  }
  GVariant *images = g_variant_lookup_value(properties, "IconPixmap",
                                            G_VARIANT_TYPE("a(iiay)"));
  if (!images || g_variant_n_children(images) != 2) {
    host->status = -4;
  } else {
    gint width, height;
    GVariant *bytes;
    g_variant_get_child(images, 0, "(ii@ay)", &width, &height, &bytes);
    gsize length;
    const guint8 *pixels = g_variant_get_fixed_array(bytes, &length, 1);
    if (width != 1 || height != 1 || length != 4 || pixels[0] != 128 ||
        pixels[1] != (host->stage == 1 ? 16 : 96) || pixels[2] != 32 ||
        pixels[3] != 64)
      host->status = -5;
    g_variant_unref(bytes);
  }
  g_clear_pointer(&images, g_variant_unref);
  g_variant_unref(properties);
  g_variant_unref(reply);
  if (host->status >= 0)
    call(host, host->menu_path, MENU, "GetLayout",
         g_variant_new("(ii@as)", 0, -1, g_variant_new_strv(NULL, 0)),
         G_VARIANT_TYPE("(u(ia{sv}av))"), layout);
}

void scoop_test_linux_tray_inspect(void *pointer, int32_t stage) {
  TestHost *host = pointer;
  host->stage = stage;
  host->status = 0;
  if (!host->service || !host->path || host->pending) {
    host->status = -30;
    return;
  }
  call(host, host->path, "org.freedesktop.DBus.Properties", "GetAll",
       g_variant_new("(s)", ITEM), G_VARIANT_TYPE("(a{sv})"), snapshot);
}

int32_t scoop_test_linux_tray_status(void *pointer) {
  return ((TestHost *)pointer)->status;
}

void scoop_test_linux_tray_destroy(void *pointer) {
  TestHost *host = pointer;
  host->closing = TRUE;
  g_cancellable_cancel(host->cancel);
  while (host->pending)
    g_main_context_iteration(NULL, TRUE);
  if (host->registration)
    g_dbus_connection_unregister_object(host->bus, host->registration);
  if (host->bus) {
    g_dbus_connection_close_sync(host->bus, NULL, NULL);
    g_object_unref(host->bus);
  }
  g_clear_pointer(&host->interface, g_dbus_node_info_unref);
  g_object_unref(host->cancel);
  g_free(host->service);
  g_free(host->path);
  g_free(host->menu_path);
  g_free(host);
}

void *scoop_test_linux_tray_host(void) {
  TestHost *host = g_new0(TestHost, 1);
  host->cancel = g_cancellable_new();
  GError *error = NULL;
  gchar *address =
      g_dbus_address_get_for_bus_sync(G_BUS_TYPE_SESSION, NULL, &error);
  if (address) {
    host->bus = g_dbus_connection_new_for_address_sync(
        address,
        G_DBUS_CONNECTION_FLAGS_AUTHENTICATION_CLIENT |
            G_DBUS_CONNECTION_FLAGS_MESSAGE_BUS_CONNECTION,
        NULL, NULL, &error);
    g_free(address);
  }
  if (host->bus) {
    host->interface = g_dbus_node_info_new_for_xml(xml, &error);
    if (host->interface)
      host->registration = g_dbus_connection_register_object(
          host->bus, WATCHER_PATH, host->interface->interfaces[0],
          &watcher_vtable, host, NULL, &error);
  }
  if (!host->registration || !scoop_test_linux_tray_cycle(host, 1)) {
    if (error)
      g_printerr("tray host: %s\n", error->message);
    g_clear_error(&error);
    scoop_test_linux_tray_destroy(host);
    return NULL;
  }
  return host;
}
