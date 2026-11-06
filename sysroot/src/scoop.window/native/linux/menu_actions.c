#include "menu.h"

void sw_menu_event(SWMenu *menu, int kind, int64_t item, int64_t window) {
  SWEvent *event = calloc(1, sizeof(*event));
  if (!event) {
    sw_fail(menu->application, ENOMEM);
    return;
  }
  event->data.kind = kind;
  sw_stamp_input(&event->data);
  event->data.a = menu->identity;
  event->data.b = item;
  event->data.c = menu->serial;
  event->data.d = window;
  sw_enqueue(menu->application, event);
}

static SWWindow *target_window(SWMenu *menu) {
  if (menu->window)
    return menu->window;
  for (SWWindow *window = menu->application->windows; window;
       window = window->next)
    if (gtk_window_is_active(GTK_WINDOW(window->window)))
      return window;
  return NULL;
}

static bool can_activate(SWMenuEntry *entry) {
  SWMenu *menu = entry->menu;
  if (menu->binding < 0 || menu->cancelled || !entry->live || !entry->enabled ||
      entry->kind >= 3)
    return false;
  SWWindow *window = target_window(menu);
  if (window &&
      (!gtk_widget_get_sensitive(window->window) || window->destroying))
    return false;
  if (entry->role >= 2 && !window)
    return false;
  if (entry->role == 2)
    return window->minimize_button;
  if (entry->role == 3)
    return window->maximize_button && window->resizable;
  if (entry->role >= 4)
    return window->ime_allowed;
  return true;
}

bool sw_menu_activate(SWMenuEntry *entry) {
  if (!can_activate(entry))
    return false;
  SWMenu *menu = entry->menu;
  SWWindow *window = target_window(menu);
  menu->selected = entry->identity;
  if (!entry->role) {
    sw_menu_event(menu, 110, entry->identity, window ? window->identity : 0);
  } else if (entry->role == 1) {
    SWEvent *event = calloc(1, sizeof(*event));
    if (event) {
      event->data.kind = 101;
      event->data.flags = 1;
      sw_enqueue(menu->application, event);
    } else
      sw_fail(menu->application, ENOMEM);
  } else if (entry->role == 2) {
    gtk_window_minimize(GTK_WINDOW(window->window));
  } else if (entry->role == 3) {
    if (gtk_window_is_maximized(GTK_WINDOW(window->window)))
      gtk_window_unmaximize(GTK_WINDOW(window->window));
    else
      gtk_window_maximize(GTK_WINDOW(window->window));
  } else {
    // Native dialogs keep their own GTK focus/action chain. This menu belongs
    // to the Scoop content window and never edits a disabled dialog owner.
    SWEvent *event = sw_event(window, 16);
    if (event) {
      event->data.a = entry->role;
      sw_enqueue(menu->application, event);
    }
  }
  sw_menu_dismiss(menu);
  return true;
}

void sw_menu_action(GSimpleAction *action, GVariant *parameter, gpointer data) {
  sw_menu_activate(data);
}

int32_t scoop_window_menu_activate(SWMenu *menu, int64_t identity) {
  SWMenuEntry *entry = sw_menu_find(menu, identity);
  return entry && sw_menu_activate(entry);
}

bool sw_menu_key_equivalent(SWWindow *window, guint symbol) {
  SWMenu *menu = window->menu_bar;
  if (!menu || window->preedit_active || (window->modifiers & SW_ALT_GRAPH))
    return false;
  uint64_t state = window->modifiers;
  int mods = ((state & SW_CONTROL) ? 1 : 0) | ((state & SW_ALT) ? 2 : 0) |
             ((state & SW_SHIFT) ? 4 : 0) | ((state & SW_SUPER) ? 8 : 0);
  symbol = gdk_keyval_to_lower(symbol);
  for (guint index = 0; index < menu->entries->len; ++index) {
    SWMenuEntry *entry = g_ptr_array_index(menu->entries, index);
    if (entry->key && entry->key == symbol && entry->modifiers == mods &&
        entry->action && can_activate(entry)) {
      g_action_group_activate_action(
          G_ACTION_GROUP(menu->actions),
          g_action_get_name(G_ACTION(entry->action)),
          entry->kind == 2 ? g_variant_new_string("on") : NULL);
      return true;
    }
  }
  return false;
}
