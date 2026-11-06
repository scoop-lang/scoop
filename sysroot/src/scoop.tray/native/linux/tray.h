#ifndef SCOOP_TRAY_LINUX_H
#define SCOOP_TRAY_LINUX_H

#include "../tray.h"
#include <errno.h>
#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gtk/gtk.h>
#include <libdbusmenu-glib/menuitem.h>
#include <libdbusmenu-glib/server.h>
#pragma clang diagnostic pop
#include <stdbool.h>
#include <string.h>

typedef struct STMenu STMenu;
typedef struct {
  STCallback callback;
  void *context;
  char *path, *menu_path, *identity, *tooltip, *watcher;
  GDBusConnection *bus;
  GDBusNodeInfo *interface;
  GCancellable *cancellable;
  guint registration, watch, host_signals, property_signals;
  gulong closed_handler;
  guint pending;
  guint64 watcher_epoch, host_revision;
  bool started, closing, available, registered, host;
  GVariant *pixmaps;
  GVariantBuilder *pending_pixmaps;
  STMenu *menu;
} STTray;

void st_emit(STTray *tray, STEvent event);
void st_signal(STTray *tray, const char *name, GVariant *parameters);
STMenu *st_menu_new(STTray *tray, GMenuModel *model, GActionGroup *actions);
void st_menu_free(STMenu *menu);

#endif
