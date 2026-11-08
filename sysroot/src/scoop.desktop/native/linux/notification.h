#ifndef SCOOP_NOTIFICATION_LINUX_H
#define SCOOP_NOTIFICATION_LINUX_H
#include "../notification/common.h"
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gio/gio.h>
#pragma clang diagnostic pop

typedef struct {
  int32_t kind;
  int64_t request;
  SNContent *content;
  char *id;
  char *service;
  gboolean checked_capabilities;
} SNOperation;
typedef struct {
  char *id, *payload;
  guint32 native;
} SNRecord;
typedef struct {
  int refs;
  gboolean closed, connecting, ready, active, capabilities_known, markup;
  int capabilities;
  char *id, *service;
  GMainContext *context;
  GDBusProxy *proxy;
  GCancellable *cancel;
  GSource *wake;
  SDCallback callback;
  void *callback_context;
  SNEvent *head, *tail;
  GQueue operations;
  GHashTable *names, *native_ids;
} SNLinux;

SNLinux *sn_linux_ref(SNLinux *owner);
void sn_linux_unref(gpointer owner);
void sn_linux_enqueue(SNLinux *owner, SNEvent *event);
void sn_linux_pump(SNLinux *owner);
void sn_linux_finish(SNLinux *owner, SNEvent *event);
void sn_linux_signals(GDBusProxy *proxy, const char *sender, const char *name,
                      GVariant *parameters, gpointer data);
void sn_linux_owner_changed(GObject *object, GParamSpec *spec, gpointer data);
void sn_linux_record(SNLinux *owner, const SNContent *content, guint32 native);
void sn_linux_record_free(gpointer data);
void sn_linux_operation_free(SNOperation *operation);
#endif
