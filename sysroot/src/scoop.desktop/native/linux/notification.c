#include "notification.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>

SNLinux *sn_linux_ref(SNLinux *owner) {
  ++owner->refs;
  return owner;
}
void sn_linux_operation_free(SNOperation *operation) {
  scoop_notification_content_destroy(operation->content);
  free(operation->id);
  g_free(operation->service);
  g_free(operation);
}
void sn_linux_unref(gpointer data) {
  SNLinux *owner = data;
  if (--owner->refs)
    return;
  while (!g_queue_is_empty(&owner->operations))
    sn_linux_operation_free(g_queue_pop_head(&owner->operations));
  while (owner->head) {
    SNEvent *event = owner->head;
    owner->head = event->next;
    scoop_notification_event_destroy(event);
  }
  g_clear_object(&owner->proxy);
  g_clear_object(&owner->cancel);
  g_hash_table_unref(owner->names);
  g_hash_table_unref(owner->native_ids);
  g_main_context_unref(owner->context);
  free(owner->id);
  g_free(owner->service);
  g_free(owner);
}
static gboolean wake(gpointer data) {
  SNLinux *owner = data;
  GSource *source = owner->wake;
  owner->wake = NULL;
  g_source_unref(source);
  if (!owner->closed && owner->callback)
    owner->callback(owner->callback_context, 0);
  return G_SOURCE_REMOVE;
}
static void schedule(SNLinux *owner) {
  if (owner->wake || !owner->callback || owner->closed)
    return;
  owner->wake = g_idle_source_new();
  g_source_set_priority(owner->wake, G_PRIORITY_DEFAULT);
  g_source_set_callback(owner->wake, wake, sn_linux_ref(owner), sn_linux_unref);
  g_source_attach(owner->wake, owner->context);
}
void sn_linux_enqueue(SNLinux *owner, SNEvent *event) {
  if (owner->closed) {
    scoop_notification_event_destroy(event);
    return;
  }
  if (owner->tail)
    owner->tail->next = event;
  else
    owner->head = event;
  owner->tail = event;
  schedule(owner);
}
void sn_linux_finish(SNLinux *owner, SNEvent *event) {
  sn_linux_operation_free(g_queue_pop_head(&owner->operations));
  owner->active = FALSE;
  sn_linux_enqueue(owner, event);
}
static void connected(GObject *object, GAsyncResult *result, gpointer data) {
  SNLinux *owner = data;
  GError *error = NULL;
  owner->proxy = g_dbus_proxy_new_for_bus_finish(result, &error);
  owner->connecting = FALSE;
  owner->ready = TRUE;
  if (owner->proxy && !owner->closed) {
    owner->service = g_dbus_proxy_get_name_owner(owner->proxy);
    g_signal_connect(owner->proxy, "g-signal", G_CALLBACK(sn_linux_signals),
                     owner);
    g_signal_connect(owner->proxy, "notify::g-name-owner",
                     G_CALLBACK(sn_linux_owner_changed), owner);
  }
  g_clear_error(&error);
  sn_linux_pump(owner);
  sn_linux_unref(owner);
}
static void connect_service(SNLinux *owner) {
  if (owner->proxy || owner->connecting || owner->closed)
    return;
  owner->connecting = TRUE;
  owner->ready = FALSE;
  g_dbus_proxy_new_for_bus(G_BUS_TYPE_SESSION,
                           G_DBUS_PROXY_FLAGS_DO_NOT_LOAD_PROPERTIES |
                               G_DBUS_PROXY_FLAGS_DO_NOT_AUTO_START,
                           NULL, "org.freedesktop.Notifications",
                           "/org/freedesktop/Notifications",
                           "org.freedesktop.Notifications", owner->cancel,
                           connected, sn_linux_ref(owner));
}
SDResult scoop_notification_create(const uint8_t *id, int64_t length) {
  char *identifier = sn_copy(id, length);
  if (!identifier)
    return (SDResult){NULL, ENOMEM};
  SNLinux *owner = g_new0(SNLinux, 1);
  owner->refs = 1;
  owner->id = identifier;
  owner->context = g_main_context_ref_thread_default();
  owner->cancel = g_cancellable_new();
  owner->names = g_hash_table_new_full(g_str_hash, g_str_equal, g_free, NULL);
  owner->native_ids = g_hash_table_new_full(g_direct_hash, g_direct_equal, NULL,
                                            sn_linux_record_free);
  g_queue_init(&owner->operations);
  connect_service(owner);
  return (SDResult){owner, 0};
}
int64_t scoop_notification_attach(void *pointer, SDCallback callback,
                                  void *context) {
  SNLinux *owner = pointer;
  owner->callback = callback;
  owner->callback_context = context;
  if (owner->head)
    schedule(owner);
  return 0;
}
void scoop_notification_destroy(void *pointer) {
  SNLinux *owner = pointer;
  owner->closed = TRUE;
  owner->callback = NULL;
  if (owner->proxy)
    g_signal_handlers_disconnect_by_data(owner->proxy, owner);
  if (owner->wake) {
    g_source_destroy(owner->wake);
    g_source_unref(owner->wake);
    owner->wake = NULL;
  }
  g_cancellable_cancel(owner->cancel);
  sn_linux_unref(owner);
}
SNEvent *scoop_notification_next_event(void *pointer) {
  SNLinux *owner = pointer;
  SNEvent *event = owner->head;
  if (event) {
    owner->head = event->next;
    if (!owner->head)
      owner->tail = NULL;
    event->next = NULL;
  }
  return event;
}
static void submit(SNLinux *owner, SNOperation *operation) {
  g_queue_push_tail(&owner->operations, operation);
  connect_service(owner);
  sn_linux_pump(owner);
}
int64_t scoop_notification_settings(void *pointer, int64_t request,
                                    int32_t permission) {
  SNOperation *operation = g_new0(SNOperation, 1);
  operation->kind = permission ? SN_PERMISSION : SN_SETTINGS;
  operation->request = request;
  submit(pointer, operation);
  return 0;
}
int64_t scoop_notification_post(void *pointer, int64_t request,
                                SNContent *content) {
  SNContent *copy = sn_content_copy(content);
  if (!copy)
    return ENOMEM;
  SNOperation *operation = g_new0(SNOperation, 1);
  operation->kind = SN_POST;
  operation->request = request;
  operation->content = copy;
  submit(pointer, operation);
  return 0;
}
int64_t scoop_notification_withdraw(void *pointer, int64_t request,
                                    const uint8_t *id, int64_t length) {
  char *copy = sn_copy(id, length);
  if (!copy)
    return ENOMEM;
  SNOperation *operation = g_new0(SNOperation, 1);
  operation->kind = SN_WITHDRAW;
  operation->request = request;
  operation->id = copy;
  submit(pointer, operation);
  return 0;
}
