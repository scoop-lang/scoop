#include "notification.h"
#include <string.h>

void sn_linux_record_free(gpointer data) {
  SNRecord *record = data;
  g_free(record->id);
  g_free(record->payload);
  g_free(record);
}
static void remove_record(SNLinux *owner, SNRecord *record) {
  if (g_hash_table_lookup(owner->names, record->id) == record)
    g_hash_table_remove(owner->names, record->id);
  g_hash_table_remove(owner->native_ids, GUINT_TO_POINTER(record->native));
}
void sn_linux_record(SNLinux *owner, const SNContent *content, guint32 native) {
  const char *id = sn_content_text(content, 0);
  SNRecord *previous = g_hash_table_lookup(owner->names, id);
  if (previous)
    remove_record(owner, previous);
  previous = g_hash_table_lookup(owner->native_ids, GUINT_TO_POINTER(native));
  if (previous)
    remove_record(owner, previous);
  SNRecord *record = g_new0(SNRecord, 1);
  record->id = g_strdup(id);
  record->payload = g_strdup(sn_content_text(content, 3));
  record->native = native;
  g_hash_table_insert(owner->names, g_strdup(id), record);
  g_hash_table_insert(owner->native_ids, GUINT_TO_POINTER(native), record);
}
void sn_linux_owner_changed(GObject *object, GParamSpec *spec, gpointer data) {
  SNLinux *owner = data;
  char *service = g_dbus_proxy_get_name_owner(owner->proxy);
  if (g_strcmp0(service, owner->service) == 0) {
    g_free(service);
    return;
  }
  g_free(owner->service);
  owner->service = service;
  owner->capabilities_known = FALSE;
  owner->capabilities = 0;
  g_hash_table_remove_all(owner->names);
  g_hash_table_remove_all(owner->native_ids);
  sn_linux_pump(owner);
}
void sn_linux_signals(GDBusProxy *proxy, const char *sender, const char *name,
                      GVariant *parameters, gpointer data) {
  SNLinux *owner = data;
  if (owner->closed || g_strcmp0(sender, owner->service) != 0)
    return;
  if (!strcmp(name, "ActionInvoked") &&
      g_variant_is_of_type(parameters, G_VARIANT_TYPE("(us)"))) {
    guint32 native;
    const char *action;
    g_variant_get(parameters, "(u&s)", &native, &action);
    SNRecord *record =
        g_hash_table_lookup(owner->native_ids, GUINT_TO_POINTER(native));
    if (!record)
      return;
    SNEvent *event = sn_event(SN_ACTIVATED, 0);
    if (strcmp(action, "default")) {
      if (!g_str_has_prefix(action, "scoop:") || !action[6]) {
        scoop_notification_event_destroy(event);
        return;
      }
      sn_event_set(event, 1, action + 6);
    }
    sn_event_set(event, 0, record->id);
    sn_event_set(event, 2, record->payload);
    sn_linux_enqueue(owner, event);
  } else if (!strcmp(name, "NotificationClosed") &&
             g_variant_is_of_type(parameters, G_VARIANT_TYPE("(uu)"))) {
    guint32 native, reason;
    g_variant_get(parameters, "(uu)", &native, &reason);
    SNRecord *record =
        g_hash_table_lookup(owner->native_ids, GUINT_TO_POINTER(native));
    if (!record)
      return;
    SNEvent *event = sn_event(SN_CLOSED, 0);
    event->data.detail = reason <= 3 ? (int32_t)reason : 0;
    sn_event_set(event, 0, record->id);
    remove_record(owner, record);
    sn_linux_enqueue(owner, event);
  }
}
