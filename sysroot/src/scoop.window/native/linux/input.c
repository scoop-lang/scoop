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
  SWEvent *event = sw_event(window, SWModifiers);
  if (event) {
    event->modifiers = value;
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
  SWEvent *event = sw_event(window, SWKeyboard);
  if (!event)
    return;
  SWKeyboardData *keyboard = &event->keyboard;
  keyboard->physical_code = code;
  keyboard->modifiers = window->modifiers;
  keyboard->logical_code = symbol;
  keyboard->pressed = down;
  keyboard->repeat = down && repeat;
  keyboard->dead = symbol >= XKB_KEY_dead_grave && symbol <= XKB_KEY_dead_greek;
  char value[64];
  int size = xkb_keysym_to_utf8(symbol, value, sizeof(value));
  if (size > 1 && (uint8_t)value[0] >= 32 && (uint8_t)value[0] != 127) {
    keyboard->text = (uint8_t *)strndup(value, size - 1);
    if (!keyboard->text) {
      sw_free_event(event);
      sw_fail(window->application, ENOMEM);
      return;
    }
    keyboard->text_length = size - 1;
  }
  if (tracked) {
    free((void *)window->pressed[code].text);
    window->pressed[code] = *keyboard;
    window->pressed[code].text =
        keyboard->text ? (uint8_t *)strndup((const char *)keyboard->text,
                                            keyboard->text_length)
                       : NULL;
    if (keyboard->text && !window->pressed[code].text) {
      window->pressed[code] = (SWKeyboardData){0};
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
  if (sw_menu_key_equivalent(window, symbol))
    return TRUE;
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
  sw_pointer(window, (SWPointerData){.kind = SWPointerEntered});
}
static void pointer_motion(GtkEventControllerMotion *controller, double x,
                           double y, gpointer data) {
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(controller), x, y);
  sw_pointer(window, (SWPointerData){.kind = SWPointerMoved});
}
static void pointer_leave(GtkEventControllerMotion *controller, gpointer data) {
  sw_pointer(data, (SWPointerData){.kind = SWPointerLeft});
}
static int pointer_button(guint button) {
  return button == 1 ? 0 : button == 3 ? 1 : button == 2 ? 2 : (int)button;
}
static void pointer_pressed(GtkGestureClick *gesture, int count, double x,
                            double y, gpointer data) {
  SWWindow *window = data;
  GdkEvent *event =
      gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(gesture));
  if (sw_frame_pointer(window, event, x, y)) {
    gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_CLAIMED);
    return;
  }
  gtk_widget_grab_focus(window->view);
  pointer_state(window, GTK_EVENT_CONTROLLER(gesture), x, y);
  window->current_event = event;
  sw_pointer(window, (SWPointerData){.kind = SWPointerPressed,
                                     .button = pointer_button(
                                         gtk_gesture_single_get_current_button(
                                             GTK_GESTURE_SINGLE(gesture))),
                                     .click_count = count});
  window->current_event = NULL;
  gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_CLAIMED);
}
static void pointer_released(GtkGestureClick *gesture, int count, double x,
                             double y, gpointer data) {
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(gesture), x, y);
  sw_pointer(window, (SWPointerData){.kind = SWPointerReleased,
                                     .button = pointer_button(
                                         gtk_gesture_single_get_current_button(
                                             GTK_GESTURE_SINGLE(gesture))),
                                     .click_count = count});
}
static void pointer_cancel(GtkGesture *gesture, GdkEventSequence *sequence,
                           gpointer data) {
  SWWindow *window = data;
  bool resetting = window->resetting_click;
  window->resetting_click = true;
  sw_cancel_pointer(window, SWPointerCaptureLost);
  window->resetting_click = resetting;
}
static gboolean scroll(GtkEventControllerScroll *controller, double dx,
                       double dy, gpointer data) {
  SWWindow *window = data;
  window->application->gdk_modifiers =
      gtk_event_controller_get_current_event_state(
          GTK_EVENT_CONTROLLER(controller));
  SWPointerData pointer = {.scale = window->snapshot.scale};
  sw_pointer_scroll(
      &pointer,
      gtk_event_controller_scroll_get_unit(controller) != GDK_SCROLL_UNIT_WHEEL,
      -dx, -dy, window->scroll_active ? SWScrollChanged : SWScrollNone,
      SWScrollNone);
  sw_pointer(window, pointer);
  return TRUE;
}
static void scroll_begin(GtkEventControllerScroll *controller, gpointer data) {
  SWWindow *window = data;
  window->scroll_active = true;
  sw_pointer(window, (SWPointerData){.kind = SWPointerPixelWheel,
                                     .phase = SWScrollBegan});
}
static void scroll_end(GtkEventControllerScroll *controller, gpointer data) {
  SWWindow *window = data;
  if (!window->scroll_active)
    return;
  window->scroll_active = false;
  sw_pointer(window, (SWPointerData){.kind = SWPointerPixelWheel,
                                     .phase = SWScrollEnded});
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
  window->click_controller = GTK_EVENT_CONTROLLER(g_object_ref(click));
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
  bool pressed = window->pressed_buttons != 0;
  window->pressed_buttons = 0;
  memset(window->pressed_clicks, 0, sizeof(window->pressed_clicks));
  if (pressed) {
    SWEvent *event = sw_event(window, SWPointer);
    if (event) {
      event->pointer.kind = SWPointerCancelled;
      event->pointer.cancel_reason = reason;
      sw_enqueue(window->application, event);
    }
  }
  if (window->scroll_active) {
    window->scroll_active = false;
    sw_pointer(window, (SWPointerData){.kind = SWPointerPixelWheel,
                                       .phase = SWScrollCancelled});
  }
  if (window->click_controller && !window->resetting_click) {
    window->resetting_click = true;
    gtk_event_controller_reset(window->click_controller);
    window->resetting_click = false;
  }
}

void sw_reset_input(SWWindow *window) {
  sw_cancel_pointer(window, SWPointerFocusLost);
  sw_cancel_composition(window);
  uint64_t locks = window->modifiers & (SW_CAPS | SW_NUM | SW_SCROLL |
                                        (1u << 16) | SW_ALT_GRAPH_KNOWN);
  for (size_t code = 0;
       code < sizeof(window->key_down) / sizeof(window->key_down[0]); ++code) {
    if (!window->key_down[code])
      continue;
    window->key_down[code] = false;
    SWEvent *event = sw_event(window, SWKeyboard);
    if (!event)
      continue;
    event->keyboard = window->pressed[code];
    window->pressed[code].text = NULL;
    event->keyboard.pressed = 0;
    event->keyboard.repeat = 0;
    event->keyboard.synthetic = 1;
    event->keyboard.modifiers = locks;
    sw_enqueue(window->application, event);
  }
  if (window->modifiers != locks) {
    window->modifiers = locks;
    SWEvent *event = sw_event(window, SWModifiers);
    if (event) {
      event->modifiers = locks;
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

void sw_pointer(SWWindow *window, SWPointerData data) {
  int kind = data.kind;
  int button = data.button;
  if (kind == SWPointerReleased && button >= 0 && button < 32 &&
      !(window->pressed_buttons & (1u << button)))
    return;
  SWEvent *event = sw_event(window, SWPointer);
  if (!event)
    return;
  data.x = window->pointer_x;
  data.y = window->pointer_y;
  data.scale = window->snapshot.scale;
  data.physical_x = data.x * data.scale;
  data.physical_y = data.y * data.scale;
  data.modifiers = sw_modifiers(window->application);
  if (kind == SWPointerEntered)
    window->pointer_inside = true;
  if (kind == SWPointerLeft)
    window->pointer_inside = false;
  if (kind == SWPointerPressed) {
    if (window->current_event)
      event->native_input = gdk_event_ref(window->current_event);
    data.click_count = data.click_count > 0 ? data.click_count : 1;
    if (button >= 0 && button < 32) {
      window->pressed_clicks[button] = data.click_count;
      window->pressed_buttons |= 1u << button;
    }
  }
  if (kind == SWPointerReleased) {
    data.click_count = data.click_count > 0 ? data.click_count : 1;
    if (button >= 0 && button < 32) {
      data.click_count = window->pressed_clicks[button];
      window->pressed_clicks[button] = 0;
      window->pressed_buttons &= ~(1u << button);
    }
  }
  event->pointer = data;
  sw_enqueue(window->application, event);
}
