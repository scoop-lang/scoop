#include "tray.h"

#define ITEM_INTERFACE "org.kde.StatusNotifierItem"
#define WATCHER_INTERFACE "org.kde.StatusNotifierWatcher"
#define WATCHER_PATH "/StatusNotifierWatcher"

static const char interface_xml[] =
    "<node><interface name='" ITEM_INTERFACE "'>"
    "<method name='Activate'><arg type='i' direction='in'/><arg type='i' "
    "direction='in'/></method>"
    "<method name='SecondaryActivate'><arg type='i' direction='in'/><arg "
    "type='i' direction='in'/></method>"
    "<method name='ContextMenu'><arg type='i' direction='in'/><arg type='i' "
    "direction='in'/></method>"
    "<method name='Scroll'><arg type='i' direction='in'/><arg type='s' "
    "direction='in'/></method>"
    "<property name='Category' type='s' access='read'/>"
    "<property name='Id' type='s' access='read'/>"
    "<property name='Title' type='s' access='read'/>"
    "<property name='Status' type='s' access='read'/>"
    "<property name='WindowId' type='u' access='read'/>"
    "<property name='IconThemePath' type='s' access='read'/>"
    "<property name='IconName' type='s' access='read'/>"
    "<property name='IconPixmap' type='a(iiay)' access='read'/>"
    "<property name='OverlayIconName' type='s' access='read'/>"
    "<property name='OverlayIconPixmap' type='a(iiay)' access='read'/>"
    "<property name='AttentionIconName' type='s' access='read'/>"
    "<property name='AttentionIconPixmap' type='a(iiay)' access='read'/>"
    "<property name='AttentionMovieName' type='s' access='read'/>"
    "<property name='ToolTip' type='(sa(iiay)ss)' access='read'/>"
    "<property name='ItemIsMenu' type='b' access='read'/>"
    "<property name='Menu' type='o' access='read'/>"
    "<signal name='NewTitle'/><signal name='NewIcon'/><signal "
    "name='NewToolTip'/>"
    "<signal name='NewAttentionIcon'/><signal name='NewOverlayIcon'/>"
    "<signal name='NewStatus'><arg type='s'/></signal>"
    "</interface></node>";

typedef struct {
  STTray *tray;
  guint64 watcher_epoch, host_revision;
} STRequest;

void st_emit(STTray *tray, STEvent event) {
  tray->callback(tray->context, &event);
}

static void availability(STTray *tray) {
  bool available = tray->registered && tray->host && !tray->closing &&
                   tray->bus && !g_dbus_connection_is_closed(tray->bus);
  if (available != tray->available) {
    tray->available = available;
    if (!tray->closing)
      st_emit(tray, (STEvent){.kind = 1, .action = available});
  }
}

void st_signal(STTray *tray, const char *name, GVariant *parameters) {
  if (tray->registration && !tray->closing &&
      !g_dbus_connection_is_closed(tray->bus))
    g_dbus_connection_emit_signal(tray->bus, NULL, tray->path, ITEM_INTERFACE,
                                  name, parameters, NULL);
}

static void menu_changed(STTray *tray) {
  if (!tray->registration || tray->closing ||
      g_dbus_connection_is_closed(tray->bus))
    return;
  GVariantBuilder properties, invalidated;
  g_variant_builder_init(&properties, G_VARIANT_TYPE("a{sv}"));
  g_variant_builder_add(
      &properties, "{sv}", "Menu",
      g_variant_new_object_path(tray->menu ? tray->menu_path : "/"));
  g_variant_builder_init(&invalidated, G_VARIANT_TYPE("as"));
  g_dbus_connection_emit_signal(
      tray->bus, NULL, tray->path, "org.freedesktop.DBus.Properties",
      "PropertiesChanged",
      g_variant_new("(sa{sv}as)", ITEM_INTERFACE, &properties, &invalidated),
      NULL);
}

static GVariant *empty_pixmaps(void) {
  return g_variant_new_array(G_VARIANT_TYPE("(iiay)"), NULL, 0);
}

static GVariant *property(GDBusConnection *connection, const gchar *sender,
                          const gchar *path, const gchar *interface,
                          const gchar *name, GError **error, gpointer data) {
  STTray *tray = data;
  if (!strcmp(name, "Category"))
    return g_variant_new_string("ApplicationStatus");
  if (!strcmp(name, "Id"))
    return g_variant_new_string(tray->identity);
  if (!strcmp(name, "Title"))
    return g_variant_new_string(tray->tooltip);
  if (!strcmp(name, "Status"))
    return g_variant_new_string("Active");
  if (!strcmp(name, "WindowId"))
    return g_variant_new_uint32(0);
  if (!strcmp(name, "IconPixmap"))
    return g_variant_ref(tray->pixmaps);
  if (!strcmp(name, "OverlayIconPixmap") ||
      !strcmp(name, "AttentionIconPixmap"))
    return empty_pixmaps();
  if (!strcmp(name, "ToolTip"))
    return g_variant_new("(s@a(iiay)ss)", "", g_variant_ref(tray->pixmaps),
                         tray->tooltip, "");
  if (!strcmp(name, "ItemIsMenu"))
    return g_variant_new_boolean(FALSE);
  if (!strcmp(name, "Menu"))
    return g_variant_new_object_path(tray->menu ? tray->menu_path : "/");
  if (!strcmp(name, "IconThemePath") || !strcmp(name, "IconName") ||
      !strcmp(name, "OverlayIconName") || !strcmp(name, "AttentionIconName") ||
      !strcmp(name, "AttentionMovieName"))
    return g_variant_new_string("");
  g_set_error(error, G_DBUS_ERROR, G_DBUS_ERROR_UNKNOWN_PROPERTY,
              "Unknown tray property: %s", name);
  return NULL;
}

static void method(GDBusConnection *connection, const gchar *sender,
                   const gchar *path, const gchar *interface, const gchar *name,
                   GVariant *parameters, GDBusMethodInvocation *invocation,
                   gpointer data) {
  STTray *tray = data;
  if (tray->closing) {
    g_dbus_method_invocation_return_error(invocation, G_DBUS_ERROR,
                                          G_DBUS_ERROR_DISCONNECTED,
                                          "Tray icon is closing");
    return;
  }
  int action = !strcmp(name, "Activate")            ? 1
               : !strcmp(name, "ContextMenu")       ? 2
               : !strcmp(name, "SecondaryActivate") ? 3
                                                    : 0;
  if (!action) {
    g_dbus_method_invocation_return_error(invocation, G_DBUS_ERROR,
                                          G_DBUS_ERROR_NOT_SUPPORTED,
                                          "Tray scrolling is not exposed");
    return;
  }
  gint x, y;
  g_variant_get(parameters, "(ii)", &x, &y);
  // SNI supplies an activation hint, not an icon rectangle or pointer identity.
  st_emit(tray,
          (STEvent){.kind = 2, .action = action, .anchor = 2, .x = x, .y = y});
  g_dbus_method_invocation_return_value(invocation, NULL);
}

static const GDBusInterfaceVTable vtable = {.method_call = method,
                                            .get_property = property};

static void finished(STTray *tray) {
  --tray->pending;
  if (tray->closing && !tray->pending)
    st_emit(tray, (STEvent){.kind = 4});
}

static STRequest *request(STTray *tray) {
  STRequest *value = g_new0(STRequest, 1);
  value->tray = tray;
  value->watcher_epoch = tray->watcher_epoch;
  value->host_revision = tray->host_revision;
  ++tray->pending;
  return value;
}

static bool current(STRequest *value) {
  return !value->tray->closing &&
         value->watcher_epoch == value->tray->watcher_epoch;
}

static void request_finished(STRequest *value) {
  STTray *tray = value->tray;
  g_free(value);
  finished(tray);
}

static void registered(GObject *source, GAsyncResult *result, gpointer data) {
  STRequest *value = data;
  GError *error = NULL;
  GVariant *reply =
      g_dbus_connection_call_finish(G_DBUS_CONNECTION(source), result, &error);
  if (current(value)) {
    value->tray->registered = reply != NULL;
    availability(value->tray);
  }
  g_clear_pointer(&reply, g_variant_unref);
  g_clear_error(&error);
  request_finished(value);
}

static void host_property(GObject *source, GAsyncResult *result,
                          gpointer data) {
  STRequest *value = data;
  GError *error = NULL;
  GVariant *reply =
      g_dbus_connection_call_finish(G_DBUS_CONNECTION(source), result, &error);
  if (current(value) && reply &&
      value->host_revision == value->tray->host_revision) {
    GVariant *property = NULL;
    g_variant_get(reply, "(v)", &property);
    if (g_variant_is_of_type(property, G_VARIANT_TYPE_BOOLEAN))
      value->tray->host = g_variant_get_boolean(property);
    g_variant_unref(property);
    availability(value->tray);
  }
  g_clear_pointer(&reply, g_variant_unref);
  g_clear_error(&error);
  request_finished(value);
}

static void query_host(STTray *tray) {
  ++tray->host_revision;
  g_dbus_connection_call(tray->bus, tray->watcher, WATCHER_PATH,
                         "org.freedesktop.DBus.Properties", "Get",
                         g_variant_new("(ss)", WATCHER_INTERFACE,
                                       "IsStatusNotifierHostRegistered"),
                         G_VARIANT_TYPE("(v)"), G_DBUS_CALL_FLAGS_NONE, -1,
                         tray->cancellable, host_property, request(tray));
}

static void appeared(GDBusConnection *connection, const gchar *name,
                     const gchar *owner, gpointer data) {
  STTray *tray = data;
  if (tray->closing)
    return;
  g_free(tray->watcher);
  tray->watcher = g_strdup(owner);
  ++tray->watcher_epoch;
  tray->registered = false;
  tray->host = false;
  availability(tray);
  g_dbus_connection_call(connection, owner, WATCHER_PATH, WATCHER_INTERFACE,
                         "RegisterStatusNotifierItem",
                         g_variant_new("(s)", tray->path), G_VARIANT_TYPE_UNIT,
                         G_DBUS_CALL_FLAGS_NONE, -1, tray->cancellable,
                         registered, request(tray));
  query_host(tray);
}

static void vanished(GDBusConnection *connection, const gchar *name,
                     gpointer data) {
  STTray *tray = data;
  g_clear_pointer(&tray->watcher, g_free);
  ++tray->watcher_epoch;
  tray->registered = false;
  tray->host = false;
  availability(tray);
}

static void host_signal(GDBusConnection *connection, const gchar *sender,
                        const gchar *path, const gchar *interface,
                        const gchar *name, GVariant *parameters,
                        gpointer data) {
  STTray *tray = data;
  if (tray->closing || g_strcmp0(sender, tray->watcher))
    return;
  if (!strcmp(name, "StatusNotifierHostRegistered"))
    tray->host = true;
  else if (!strcmp(name, "StatusNotifierHostUnregistered"))
    tray->host = false;
  else
    return;
  ++tray->host_revision;
  availability(tray);
}

static void property_signal(GDBusConnection *connection, const gchar *sender,
                            const gchar *path, const gchar *interface,
                            const gchar *name, GVariant *parameters,
                            gpointer data) {
  STTray *tray = data;
  if (tray->closing || g_strcmp0(sender, tray->watcher))
    return;
  const char *changed_interface;
  GVariant *changed, *invalidated;
  g_variant_get(parameters, "(&s@a{sv}@as)", &changed_interface, &changed,
                &invalidated);
  if (!strcmp(changed_interface, WATCHER_INTERFACE)) {
    gboolean value;
    if (g_variant_lookup(changed, "IsStatusNotifierHostRegistered", "b",
                         &value)) {
      tray->host = value;
      ++tray->host_revision;
      availability(tray);
    } else {
      GVariantIter iterator;
      const char *property;
      g_variant_iter_init(&iterator, invalidated);
      while (g_variant_iter_next(&iterator, "&s", &property))
        if (!strcmp(property, "IsStatusNotifierHostRegistered"))
          query_host(tray);
    }
  }
  g_variant_unref(changed);
  g_variant_unref(invalidated);
}

static void disconnected(GDBusConnection *connection, gboolean remote,
                         GError *error, gpointer data) {
  STTray *tray = data;
  ++tray->watcher_epoch;
  tray->registered = false;
  tray->host = false;
  availability(tray);
}

static void connected(GObject *source, GAsyncResult *result, gpointer data) {
  STTray *tray = data;
  GError *error = NULL;
  GDBusConnection *bus = g_bus_get_finish(result, &error);
  if (bus && !tray->closing) {
    tray->bus = bus;
    g_dbus_connection_set_exit_on_close(bus, FALSE);
    tray->registration = g_dbus_connection_register_object(
        bus, tray->path, tray->interface->interfaces[0], &vtable, tray, NULL,
        &error);
    if (tray->registration) {
      tray->closed_handler =
          g_signal_connect(bus, "closed", G_CALLBACK(disconnected), tray);
      tray->host_signals = g_dbus_connection_signal_subscribe(
          bus, WATCHER_INTERFACE, WATCHER_INTERFACE, NULL, WATCHER_PATH, NULL,
          G_DBUS_SIGNAL_FLAGS_NONE, host_signal, tray, NULL);
      tray->property_signals = g_dbus_connection_signal_subscribe(
          bus, WATCHER_INTERFACE, "org.freedesktop.DBus.Properties",
          "PropertiesChanged", WATCHER_PATH, WATCHER_INTERFACE,
          G_DBUS_SIGNAL_FLAGS_NONE, property_signal, tray, NULL);
      tray->watch = g_bus_watch_name_on_connection(
          bus, WATCHER_INTERFACE, G_BUS_NAME_WATCHER_FLAGS_NONE, appeared,
          vanished, tray, NULL);
    } else {
      st_emit(tray, (STEvent){.kind = 6, .error = error ? error->code : EIO});
    }
  } else if (bus) {
    g_object_unref(bus);
  }
  g_clear_error(&error);
  finished(tray);
}

STResult scoop_tray_create(STCallback callback, void *context,
                           int64_t identity) {
  STTray *tray = g_new0(STTray, 1);
  tray->callback = callback;
  tray->context = context;
  tray->path =
      g_strdup_printf("/StatusNotifierItem/%" G_GINT64_FORMAT, identity);
  tray->menu_path = g_strdup_printf("/Menu/%" G_GINT64_FORMAT, identity);
  tray->identity = g_strdup_printf("scoop-tray-%" G_GINT64_FORMAT, identity);
  tray->tooltip = g_strdup("");
  tray->pixmaps = g_variant_ref_sink(empty_pixmaps());
  tray->cancellable = g_cancellable_new();
  GError *error = NULL;
  tray->interface = g_dbus_node_info_new_for_xml(interface_xml, &error);
  if (!tray->interface) {
    int64_t code = error->code;
    g_error_free(error);
    g_free(tray->path);
    g_free(tray->menu_path);
    g_free(tray->identity);
    g_free(tray->tooltip);
    g_variant_unref(tray->pixmaps);
    g_object_unref(tray->cancellable);
    g_free(tray);
    return (STResult){.error = code};
  }
  return (STResult){tray, 0};
}

int64_t scoop_tray_start(void *pointer) {
  STTray *tray = pointer;
  tray->started = true;
  ++tray->pending;
  g_bus_get(G_BUS_TYPE_SESSION, tray->cancellable, connected, tray);
  return 0;
}

int32_t scoop_tray_available(void *pointer) {
  return ((STTray *)pointer)->available;
}
int32_t scoop_tray_busy(void *pointer) {
  STTray *tray = pointer;
  return tray->closing && tray->pending != 0;
}
int32_t scoop_tray_capabilities(void *pointer) { return 2 | 4 | 64 | 128; }
STGeometry scoop_tray_geometry(void *pointer) {
  return (STGeometry){.error = -1};
}
STSelection scoop_tray_show_menu(void *pointer) {
  return (STSelection){.error = -1};
}

void scoop_tray_cancel(void *pointer) {
  // The SNI host owns remote menu presentation; this backend has no local loop.
  (void)pointer;
}

int64_t scoop_tray_image_begin(void *pointer, double width, double height,
                               int32_t template) {
  STTray *tray = pointer;
  g_clear_pointer(&tray->pending_pixmaps, g_variant_builder_unref);
  tray->pending_pixmaps = g_variant_builder_new(G_VARIANT_TYPE("a(iiay)"));
  return 0;
}

int64_t scoop_tray_image_add(void *pointer, int32_t width, int32_t height,
                             const uint8_t *pixels) {
  STTray *tray = pointer;
  if (width <= 0 || height <= 0 ||
      (size_t)width > SIZE_MAX / 4 / (size_t)height)
    return EOVERFLOW;
  size_t length = (size_t)width * (size_t)height * 4;
  guint8 *argb = g_try_malloc(length);
  if (!argb)
    return ENOMEM;
  for (size_t index = 0; index < length; index += 4) {
    argb[index] = pixels[index + 3];
    argb[index + 1] = pixels[index];
    argb[index + 2] = pixels[index + 1];
    argb[index + 3] = pixels[index + 2];
  }
  GVariant *bytes = g_variant_new_from_data(G_VARIANT_TYPE("ay"), argb, length,
                                            TRUE, g_free, argb);
  g_variant_builder_add(tray->pending_pixmaps, "(ii@ay)", width, height, bytes);
  return 0;
}

int64_t scoop_tray_image_commit(void *pointer) {
  STTray *tray = pointer;
  GVariant *next =
      g_variant_ref_sink(g_variant_builder_end(tray->pending_pixmaps));
  g_clear_pointer(&tray->pending_pixmaps, g_variant_builder_unref);
  g_variant_unref(tray->pixmaps);
  tray->pixmaps = next;
  st_signal(tray, "NewIcon", NULL);
  st_signal(tray, "NewToolTip", NULL);
  return 0;
}

int64_t scoop_tray_tooltip(void *pointer, const uint8_t *text, int64_t length) {
  STTray *tray = pointer;
  g_free(tray->tooltip);
  tray->tooltip = g_strndup((const char *)text, (gsize)length);
  st_signal(tray, "NewTitle", NULL);
  st_signal(tray, "NewToolTip", NULL);
  return 0;
}

int64_t scoop_tray_menu(void *pointer, void *model, void *actions) {
  STTray *tray = pointer;
  st_menu_free(tray->menu);
  tray->menu =
      model ? st_menu_new(tray, G_MENU_MODEL(model), G_ACTION_GROUP(actions))
            : NULL;
  menu_changed(tray);
  return 0;
}

void scoop_tray_request_close(void *pointer) {
  STTray *tray = pointer;
  tray->closing = true;
  tray->available = false;
  g_cancellable_cancel(tray->cancellable);
  if (tray->watch) {
    g_bus_unwatch_name(tray->watch);
    tray->watch = 0;
  }
  if (tray->host_signals) {
    g_dbus_connection_signal_unsubscribe(tray->bus, tray->host_signals);
    tray->host_signals = 0;
  }
  if (tray->property_signals) {
    g_dbus_connection_signal_unsubscribe(tray->bus, tray->property_signals);
    tray->property_signals = 0;
  }
  if (tray->closed_handler) {
    g_signal_handler_disconnect(tray->bus, tray->closed_handler);
    tray->closed_handler = 0;
  }
  if (tray->registration) {
    g_dbus_connection_unregister_object(tray->bus, tray->registration);
    tray->registration = 0;
  }
}

void scoop_tray_destroy(void *pointer) {
  STTray *tray = pointer;
  st_menu_free(tray->menu);
  g_clear_object(&tray->bus);
  g_object_unref(tray->cancellable);
  g_dbus_node_info_unref(tray->interface);
  g_variant_unref(tray->pixmaps);
  g_clear_pointer(&tray->pending_pixmaps, g_variant_builder_unref);
  g_free(tray->path);
  g_free(tray->menu_path);
  g_free(tray->identity);
  g_free(tray->tooltip);
  g_free(tray->watcher);
  g_free(tray);
}
