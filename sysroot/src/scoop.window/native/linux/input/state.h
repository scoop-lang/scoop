#ifndef SCOOP_WINDOW_LINUX_INPUT_STATE_H
#define SCOOP_WINDOW_LINUX_INPUT_STATE_H

#include "../../input.h"
#include <gtk/gtk.h>

typedef struct {
  int layout;
  guint gdk_modifiers;
  bool alt_graph, physical_keys_known;
  bool physical_keys[768];
} SWApplicationInputState;

typedef struct {
  double pointer_x, pointer_y;
  GtkEventController *click_controller;
  bool resetting_click, scroll_active;
  int pressed_clicks[32];
  uint64_t modifiers;
  uint32_t pressed_buttons;
  SWKeyboardData pressed[768];
  bool key_down[768];
  bool cursor_visible, pointer_inside;
  int cursor_icon;
} SWInputState;

#endif
