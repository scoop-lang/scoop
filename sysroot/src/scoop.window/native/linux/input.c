#include "window.h"
#include <xkbcommon/xkbcommon-keysyms.h>

uint64_t sw_modifiers(SWApplication *app) {
  guint state = app->gdk_modifiers;
  uint64_t result = SW_ALT_GRAPH_KNOWN;
  if (state & GDK_SHIFT_MASK)
    result |= SW_SHIFT;
  if (state & GDK_CONTROL_MASK)
    result |= SW_CONTROL;
  if (state & GDK_ALT_MASK)
    result |= SW_ALT;
  if (state & (GDK_SUPER_MASK | GDK_META_MASK))
    result |= SW_SUPER;
  if (state & GDK_LOCK_MASK)
    result |= SW_CAPS;
  if (state & (1u << 4))
    result |= SW_NUM;
  if (app->alt_graph || (state & (1u << 7)))
    result |= SW_ALT_GRAPH;
  if (app->physical_keys_known) {
    static const unsigned sides[] = {42, 54, 29, 97, 56, 100, 125, 126};
    result |= 1u << 16;
    for (unsigned index = 0; index < 8; ++index)
      if (app->physical_keys[sides[index]])
        result |= 1u << (8 + index);
  }
  return result;
}
void sw_update_modifiers(SWWindow *window) {
  uint64_t value = sw_modifiers(window->application);
  if (window->modifiers == value)
    return;
  window->modifiers = value;
  SWEvent *event = sw_event(window, 11);
  if (event) {
    event->data.a = value;
    sw_enqueue(window->application, event);
  }
}
static void key_event(SWWindow *window, guint symbol, guint keycode,
                      bool down) {
  if (keycode < 8)
    return;
  guint code = keycode - 8;
  bool tracked = code < G_N_ELEMENTS(window->key_down);
  if (!down && (!tracked || !window->key_down[code]))
    return;
  bool repeat = tracked && window->key_down[code];
  SWEvent *event = sw_event(window, 10);
  if (!event)
    return;
  event->data.a = code;
  event->data.b = window->modifiers;
  event->data.c = symbol;
  event->data.flags = (down ? 1 : 0) | (down && repeat ? 2 : 0);
  if (symbol >= XKB_KEY_dead_grave && symbol <= XKB_KEY_dead_greek)
    event->data.flags |= 8;
  char value[64];
  int size = xkb_keysym_to_utf8(symbol, value, sizeof(value));
  if (size > 1 && (uint8_t)value[0] >= 32 && (uint8_t)value[0] != 127) {
    event->data.text = (uint8_t *)strndup(value, size - 1);
    if (!event->data.text) {
      sw_free_event(event);
      sw_fail(window->application, ENOMEM);
      return;
    }
    event->data.text_length = size - 1;
  }
  if (tracked) {
    free((void *)window->pressed[code].text);
    window->pressed[code] = event->data;
    window->pressed[code].text =
        event->data.text ? (uint8_t *)strndup((const char *)event->data.text,
                                              event->data.text_length)
                         : NULL;
    if (event->data.text && !window->pressed[code].text) {
      window->pressed[code] = (SWEventData){0};
      sw_free_event(event);
      sw_fail(window->application, ENOMEM);
      return;
    }
    window->key_down[code] = down;
  }
  sw_enqueue(window->application, event);
}
static void key_state(SWWindow *window, GtkEventController *controller,
                      guint symbol, guint code, GdkModifierType state,
                      bool down) {
  SWApplication *app = window->application;
  app->gdk_modifiers = state;
  GdkEvent *event = gtk_event_controller_get_current_event(controller);
  if (event)
    app->layout = gdk_key_event_get_layout(event);
  if (symbol == GDK_KEY_ISO_Level3_Shift || symbol == GDK_KEY_ISO_Level5_Shift)
    app->alt_graph = down;
  if (!app->physical_keys_known &&
      !(state & (GDK_SHIFT_MASK | GDK_CONTROL_MASK | GDK_ALT_MASK |
                 GDK_SUPER_MASK | GDK_META_MASK))) {
    memset(app->physical_keys, 0, sizeof(app->physical_keys));
    app->physical_keys_known = true;
  }
  if (code >= 8 && code - 8 < G_N_ELEMENTS(app->physical_keys)) {
    app->physical_keys[code - 8] = down;
  }
  sw_update_modifiers(window);
}
static gboolean key_pressed(GtkEventControllerKey *controller, guint symbol,
                            guint code, GdkModifierType state, gpointer data) {
  SWWindow *window = data;
  key_state(window, GTK_EVENT_CONTROLLER(controller), symbol, code, state,
            true);
  bool composing = window->preedit_active;
  if (!(composing && (symbol == GDK_KEY_Return || symbol == GDK_KEY_Escape)))
    key_event(window, symbol, code, true);
  GdkEvent *event =
      gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller));
  if (event)
    gtk_im_context_filter_keypress(sw_ime_context(window), event);
  return TRUE;
}
static void key_released(GtkEventControllerKey *controller, guint symbol,
                         guint code, GdkModifierType state, gpointer data) {
  SWWindow *window = data;
  key_state(window, GTK_EVENT_CONTROLLER(controller), symbol, code, state,
            false);
  key_event(window, symbol, code, false);
  GdkEvent *event =
      gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller));
  if (event)
    gtk_im_context_filter_keypress(sw_ime_context(window), event);
}
static gboolean modifiers(GtkEventControllerKey *controller,
                          GdkModifierType state, gpointer data) {
  SWWindow *window = data;
  window->application->gdk_modifiers = state;
  sw_update_modifiers(window);
  return FALSE;
}
static void pointer_state(SWWindow *window, GtkEventController *controller,
                          double x, double y) {
  window->pointer_x = x;
  window->pointer_y = y;
  window->application->gdk_modifiers =
      gtk_event_controller_get_current_event_state(controller);
}
static void pointer_enter(GtkEventControllerMotion *controller, double x,
                          double y, gpointer data) {
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(controller), x, y);
  sw_pointer(window, 0, 0, 0, 0, 0);
}
static void pointer_motion(GtkEventControllerMotion *controller, double x,
                           double y, gpointer data) {
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(controller), x, y);
  sw_pointer(window, 2, 0, 0, 0, 0);
}
static void pointer_leave(GtkEventControllerMotion *controller, gpointer data) {
  sw_pointer(data, 1, 0, 0, 0, 0);
}
static int pointer_button(guint button) {
  return button == 1 ? 0 : button == 3 ? 1 : button == 2 ? 2 : (int)button;
}
static void pointer_pressed(GtkGestureClick *gesture, int count, double x,
                            double y, gpointer data) {
  SWWindow *window = data;
  gtk_widget_grab_focus(window->view);
  pointer_state(window, GTK_EVENT_CONTROLLER(gesture), x, y);
  sw_pointer(window, 3,
             pointer_button(gtk_gesture_single_get_current_button(
                 GTK_GESTURE_SINGLE(gesture))),
             0, 0, 0);
  gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_CLAIMED);
}
static void pointer_released(GtkGestureClick *gesture, int count, double x,
                             double y, gpointer data) {
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(gesture), x, y);
  sw_pointer(window, 4,
             pointer_button(gtk_gesture_single_get_current_button(
                 GTK_GESTURE_SINGLE(gesture))),
             0, 0, 0);
}
static void pointer_cancel(GtkGesture *gesture, GdkEventSequence *sequence,
                           gpointer data) {
  sw_cancel_pointer(data, 1);
}
static gboolean scroll(GtkEventControllerScroll *controller, double dx,
                       double dy, gpointer data) {
  SWWindow *window = data;
  window->application->gdk_modifiers =
      gtk_event_controller_get_current_event_state(
          GTK_EVENT_CONTROLLER(controller));
  sw_pointer(window,
             gtk_event_controller_scroll_get_unit(controller) ==
                     GDK_SCROLL_UNIT_WHEEL
                 ? 5
                 : 6,
             0, dx, dy, 1);
  return TRUE;
}
static void scroll_begin(GtkEventControllerScroll *controller, gpointer data) {
  sw_pointer(data, 6, 0, 0, 0, 0);
}
static void scroll_end(GtkEventControllerScroll *controller, gpointer data) {
  sw_pointer(data, 6, 0, 0, 0, 2);
}
static void focus_enter(GtkEventControllerFocus *controller, gpointer data) {
  sw_ime_update(data);
}
static void focus_leave(GtkEventControllerFocus *controller, gpointer data) {
  SWWindow *window = data;
  sw_reset_input(window);
  sw_ime_update(window);
}
void sw_input_install(SWWindow *window) {
  GtkEventController *keyboard = gtk_event_controller_key_new();
  g_signal_connect(keyboard, "key-pressed", G_CALLBACK(key_pressed), window);
  g_signal_connect(keyboard, "key-released", G_CALLBACK(key_released), window);
  g_signal_connect(keyboard, "modifiers", G_CALLBACK(modifiers), window);
  gtk_widget_add_controller(window->view, keyboard);
  GtkEventController *motion = gtk_event_controller_motion_new();
  g_signal_connect(motion, "enter", G_CALLBACK(pointer_enter), window);
  g_signal_connect(motion, "motion", G_CALLBACK(pointer_motion), window);
  g_signal_connect(motion, "leave", G_CALLBACK(pointer_leave), window);
  gtk_widget_add_controller(window->view, motion);
  GtkGesture *click = gtk_gesture_click_new();
  gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(click), 0);
  g_signal_connect(click, "pressed", G_CALLBACK(pointer_pressed), window);
  g_signal_connect(click, "released", G_CALLBACK(pointer_released), window);
  g_signal_connect(click, "cancel", G_CALLBACK(pointer_cancel), window);
  gtk_widget_add_controller(window->view, GTK_EVENT_CONTROLLER(click));
  GtkEventController *wheel =
      gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_BOTH_AXES);
  g_signal_connect(wheel, "scroll", G_CALLBACK(scroll), window);
  g_signal_connect(wheel, "scroll-begin", G_CALLBACK(scroll_begin), window);
  g_signal_connect(wheel, "scroll-end", G_CALLBACK(scroll_end), window);
  gtk_widget_add_controller(window->view, wheel);
  GtkEventController *focus = gtk_event_controller_focus_new();
  g_signal_connect(focus, "enter", G_CALLBACK(focus_enter), window);
  g_signal_connect(focus, "leave", G_CALLBACK(focus_leave), window);
  gtk_widget_add_controller(window->view, focus);
}

void sw_cancel_pointer(SWWindow *window, int reason) {
  if (!window->pressed_buttons)
    return;
  window->pressed_buttons = 0;
  SWEvent *event = sw_event(window, 12);
  if (event) {
    event->data.flags = 7;
    event->data.a = reason;
    sw_enqueue(window->application, event);
  }
}

void sw_reset_input(SWWindow *window) {
  sw_cancel_pointer(window, 0);
  sw_cancel_composition(window);
  uint64_t locks = window->modifiers & (SW_CAPS | SW_NUM | SW_SCROLL |
                                        (1u << 16) | SW_ALT_GRAPH_KNOWN);
  for (size_t code = 0;
       code < sizeof(window->key_down) / sizeof(window->key_down[0]); ++code) {
    if (!window->key_down[code])
      continue;
    window->key_down[code] = false;
    SWEvent *event = sw_event(window, 10);
    if (!event)
      continue;
    event->data = window->pressed[code];
    window->pressed[code].text = NULL;
    event->data.flags = 4;
    event->data.b = locks;
    sw_stamp_input(&event->data);
    sw_enqueue(window->application, event);
  }
  if (window->modifiers != locks) {
    window->modifiers = locks;
    SWEvent *event = sw_event(window, 11);
    if (event) {
      event->data.a = locks;
      sw_enqueue(window->application, event);
    }
  }
}

void sw_focus(SWWindow *window, bool focused) {
  if (window->snapshot.focused == focused)
    return;
  if (!focused) {
    sw_reset_input(window);
    window->application->physical_keys_known = false;
    window->application->alt_graph = false;
  }
  window->snapshot.focused = focused;
  sw_ime_update(window);
  if (window->configured)
    sw_emit(window, 4);
}

void sw_pointer(SWWindow *window, int kind, int64_t button, double dx,
                double dy, int phase) {
  SWEvent *event = sw_event(window, 12);
  if (!event)
    return;
  SWEventData *data = &event->data;
  data->flags = kind;
  data->x = window->pointer_x;
  data->y = window->pointer_y;
  data->scale = window->snapshot.scale;
  data->z = data->x * data->scale;
  data->w = data->y * data->scale;
  data->b = sw_modifiers(window->application);
  data->a = button;
  data->dx = dx;
  data->dy = dy;
  if (kind == 0)
    window->pointer_inside = true;
  if (kind == 1)
    window->pointer_inside = false;
  if (kind == 3) {
    double now = sw_now();
    bool repeated = now - window->click_time < 0.5 &&
                    button == window->click_button &&
                    fabs(data->z - window->click_x) < 5 &&
                    fabs(data->w - window->click_y) < 5;
    window->click_count =
        repeated && window->click_count < INT_MAX ? window->click_count + 1 : 1;
    window->click_time = now;
    window->click_button = button;
    window->click_x = data->z;
    window->click_y = data->w;
    data->c = window->click_count;
    if (button >= 0 && button < 32)
      window->pressed_buttons |= 1u << button;
  }
  if (kind == 4) {
    data->c = window->click_count;
    if (button >= 0 && button < 32)
      window->pressed_buttons &= ~(1u << button);
  }
  if (kind == 5 || kind == 6) {
    data->a = phase;
    data->c = 0;
  }
  sw_enqueue(window->application, event);
}
