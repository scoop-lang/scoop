#include "notification.h"
#include <string.h>

static gboolean settings_operation(const SNOperation *operation) {
  return operation->kind == SN_SETTINGS || operation->kind == SN_PERMISSION;
}
static SNEvent *unavailable(SNOperation *operation, GError *error) {
  SNEvent *event = sn_event(operation->kind, operation->request);
  if (settings_operation(operation))
    event->data.detail = SN_SERVICE_UNAVAILABLE;
  else
    sn_event_error(event, SN_UNAVAILABLE, error ? error->code : 0,
                   error ? error->message : NULL);
  return event;
}
static void done(GObject *object, GAsyncResult *result, gpointer data) {
  SNLinux *owner = data;
  SNOperation *operation = g_queue_peek_head(&owner->operations);
  GError *error = NULL;
  GVariant *reply =
      g_dbus_proxy_call_finish(G_DBUS_PROXY(object), result, &error);
  SNEvent *event;
  if (owner->closed || !reply ||
      g_strcmp0(operation->service, owner->service)) {
    event = unavailable(operation, error);
  } else {
    event = sn_event(operation->kind, operation->request);
    if (operation->kind == SN_POST) {
      guint32 id = 0;
      if (g_variant_is_of_type(reply, G_VARIANT_TYPE("(u)")))
        g_variant_get(reply, "(u)", &id);
      if (id)
        sn_linux_record(owner, operation->content, id);
      else
        sn_event_error(event, SN_NATIVE_FAILURE, 0,
                       "invalid notification service id");
    } else {
      g_hash_table_remove(owner->names, operation->id);
    }
  }
  g_clear_pointer(&reply, g_variant_unref);
  g_clear_error(&error);
  sn_linux_finish(owner, event);
  sn_linux_pump(owner);
  sn_linux_unref(owner);
}
static void call(SNLinux *owner, SNOperation *operation, const char *method,
                 GVariant *arguments) {
  g_free(operation->service);
  operation->service = g_strdup(owner->service);
  g_dbus_proxy_call(owner->proxy, method, arguments,
                    G_DBUS_CALL_FLAGS_NO_AUTO_START, -1, owner->cancel, done,
                    sn_linux_ref(owner));
}
static void execute(SNLinux *owner) {
  SNOperation *operation = g_queue_peek_head(&owner->operations);
  if (settings_operation(operation)) {
    SNEvent *event = sn_event(operation->kind, operation->request);
    event->data.detail = SN_AVAILABLE;
    event->data.capabilities = owner->capabilities;
    sn_linux_finish(owner, event);
  } else if (operation->kind == SN_WITHDRAW) {
    SNRecord *record = g_hash_table_lookup(owner->names, operation->id);
    if (!record) {
      SNEvent *event = sn_event(SN_WITHDRAW, operation->request);
      event->data.detail = 1;
      sn_linux_finish(owner, event);
    } else
      call(owner, operation, "CloseNotification",
           g_variant_new("(u)", record->native));
  } else {
    SNContent *content = operation->content;
    if (content->count && !(owner->capabilities & SN_ACTIONS)) {
      SNEvent *event = sn_event(SN_POST, operation->request);
      sn_event_error(event, SN_UNSUPPORTED, 0,
                     "notification service does not support actions");
      sn_linux_finish(owner, event);
      return;
    }
    GVariantBuilder actions, hints;
    g_variant_builder_init(&actions, G_VARIANT_TYPE("as"));
    if (owner->capabilities & SN_ACTIONS) {
      g_variant_builder_add(&actions, "s", "default");
      g_variant_builder_add(&actions, "s", sn_content_text(content, 1));
      for (size_t index = 0; index < content->count; ++index) {
        char *key = g_strconcat("scoop:", content->actions[index].id, NULL);
        g_variant_builder_add(&actions, "s", key);
        g_variant_builder_add(&actions, "s", content->actions[index].title);
        g_free(key);
      }
    }
    g_variant_builder_init(&hints, G_VARIANT_TYPE("a{sv}"));
    g_variant_builder_add(&hints, "{sv}", "desktop-entry",
                          g_variant_new_string(owner->id));
    if (content->silent)
      g_variant_builder_add(&hints, "{sv}", "suppress-sound",
                            g_variant_new_boolean(TRUE));
    const char *text = sn_content_text(content, 2);
    char *body =
        owner->markup ? g_markup_escape_text(text, -1) : g_strdup(text);
    SNRecord *previous =
        g_hash_table_lookup(owner->names, sn_content_text(content, 0));
    call(owner, operation, "Notify",
         g_variant_new("(susss@as@a{sv}i)", owner->id,
                       previous ? previous->native : 0, "",
                       sn_content_text(content, 1), body,
                       g_variant_builder_end(&actions),
                       g_variant_builder_end(&hints), -1));
    g_free(body);
  }
}
static void capabilities(GObject *object, GAsyncResult *result, gpointer data) {
  SNLinux *owner = data;
  SNOperation *operation = g_queue_peek_head(&owner->operations);
  GError *error = NULL;
  GVariant *reply =
      g_dbus_proxy_call_finish(G_DBUS_PROXY(object), result, &error);
  if (owner->closed || !reply ||
      !g_variant_is_of_type(reply, G_VARIANT_TYPE("(as)")) ||
      g_strcmp0(operation->service, owner->service)) {
    sn_linux_finish(owner, unavailable(operation, error));
  } else {
    owner->capabilities = SN_REPLACE | SN_CAN_WITHDRAW | SN_CLOSE_EVENTS;
    owner->markup = FALSE;
    GVariantIter *values;
    const char *value;
    g_variant_get(reply, "(as)", &values);
    while (g_variant_iter_next(values, "&s", &value)) {
      if (!strcmp(value, "actions"))
        owner->capabilities |= SN_ACTIONS | SN_ACTIVATION;
      if (!strcmp(value, "body-markup"))
        owner->markup = TRUE;
    }
    g_variant_iter_free(values);
    owner->capabilities_known = TRUE;
    operation->checked_capabilities = TRUE;
    execute(owner);
  }
  g_clear_pointer(&reply, g_variant_unref);
  g_clear_error(&error);
  sn_linux_pump(owner);
  sn_linux_unref(owner);
}
void sn_linux_pump(SNLinux *owner) {
  if (owner->closed || owner->active || !owner->ready)
    return;
  while (!g_queue_is_empty(&owner->operations) && !owner->active &&
         !owner->closed) {
    SNOperation *operation = g_queue_peek_head(&owner->operations);
    owner->active = TRUE;
    if (!owner->proxy || !owner->service) {
      sn_linux_finish(owner, unavailable(operation, NULL));
    } else if (!operation->checked_capabilities &&
               (settings_operation(operation) || !owner->capabilities_known)) {
      g_free(operation->service);
      operation->service = g_strdup(owner->service);
      g_dbus_proxy_call(owner->proxy, "GetCapabilities", NULL,
                        G_DBUS_CALL_FLAGS_NO_AUTO_START, -1, owner->cancel,
                        capabilities, sn_linux_ref(owner));
    } else
      execute(owner);
  }
}
