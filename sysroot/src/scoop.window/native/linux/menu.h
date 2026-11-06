#ifndef SCOOP_WINDOW_LINUX_MENU_H
#define SCOOP_WINDOW_LINUX_MENU_H

#include "window.h"

typedef struct {
  int32_t key, modifiers;
  int64_t error;
} SWMenuKey;

typedef struct SWMenuEntry {
  SWMenu *menu;
  int64_t identity;
  int32_t kind, role, modifiers;
  guint key;
  bool enabled, live;
  GMenuItem *item;
  GMenu *children;
  GPtrArray *entries;
  GSimpleAction *action;
  size_t image_pixels;
} SWMenuEntry;

struct SWMenu {
  SWApplication *application;
  SWWindow *window;
  GMenu *model;
  GSimpleActionGroup *actions;
  GPtrArray *entries, *roots;
  GtkWidget *bar, *popup;
  int64_t identity, serial, selected;
  int32_t binding;
  bool presenting, external_tracking, cancelled;
};

SWMenuEntry *sw_menu_find(SWMenu *menu, int64_t identity);
void sw_menu_event(SWMenu *menu, int kind, int64_t item, int64_t window);
bool sw_menu_activate(SWMenuEntry *entry);
void sw_menu_action(GSimpleAction *action, GVariant *parameter, gpointer data);
void sw_menu_watch(SWMenu *menu, GtkWidget *widget, bool connect);
bool sw_menu_tracking(GtkWidget *widget);
void sw_menu_dismiss(SWMenu *menu);

#endif
