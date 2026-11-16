#include "../window.h"
#include <xkbcommon/xkbcommon-keysyms.h>

uint64_t sw_modifiers(SWApplication *app) {
  guint state = app->input.gdk_modifiers;
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
  if (app->input.alt_graph || (state & (1u << 7)))
    result |= SW_ALT_GRAPH;
  if (app->input.physical_keys_known) {
    static const unsigned sides[] = {42, 54, 29, 97, 56, 100, 125, 126};
    result |= 1u << 16;
    for (unsigned index = 0; index < 8; ++index)
      if (app->input.physical_keys[sides[index]])
        result |= 1u << (8 + index);
  }
  return result;
}
void sw_update_modifiers(SWWindow *window) {
  uint64_t value = sw_modifiers(window->application);
  if (window->input.modifiers == value)
    return;
  window->input.modifiers = value;
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
  bool tracked = code < G_N_ELEMENTS(window->input.key_down);
  if (!down && (!tracked || !window->input.key_down[code]))
    return;
  bool repeat = tracked && window->input.key_down[code];
  SWEvent *event = sw_event(window, SWKeyboard);
  if (!event)
    return;
  SWKeyboardData *keyboard = &event->keyboard;
  keyboard->physical_code = code;
  keyboard->modifiers = window->input.modifiers;
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
    free((void *)window->input.pressed[code].text);
    window->input.pressed[code] = *keyboard;
    window->input.pressed[code].text =
        keyboard->text ? (uint8_t *)strndup((const char *)keyboard->text,
                                            keyboard->text_length)
                       : NULL;
    if (keyboard->text && !window->input.pressed[code].text) {
      window->input.pressed[code] = (SWKeyboardData){0};
      sw_free_event(event);
      sw_fail(window->application, ENOMEM);
      return;
    }
    window->input.key_down[code] = down;
  }
  sw_enqueue(window->application, event);
}
static void key_state(SWWindow *window, GtkEventController *controller,
                      guint symbol, guint code, GdkModifierType state,
                      bool down) {
  SWApplication *app = window->application;
  app->input.gdk_modifiers = state;
  GdkEvent *event = gtk_event_controller_get_current_event(controller);
  if (event)
    app->input.layout = gdk_key_event_get_layout(event);
  if (symbol == GDK_KEY_ISO_Level3_Shift || symbol == GDK_KEY_ISO_Level5_Shift)
    app->input.alt_graph = down;
  if (!app->input.physical_keys_known &&
      !(state & (GDK_SHIFT_MASK | GDK_CONTROL_MASK | GDK_ALT_MASK |
                 GDK_SUPER_MASK | GDK_META_MASK))) {
    memset(app->input.physical_keys, 0, sizeof(app->input.physical_keys));
    app->input.physical_keys_known = true;
  }
  if (code >= 8 && code - 8 < G_N_ELEMENTS(app->input.physical_keys)) {
    app->input.physical_keys[code - 8] = down;
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
  window->application->input.gdk_modifiers = state;
  sw_update_modifiers(window);
  return FALSE;
}

void sw_keyboard_install(SWWindow *window) {
  GtkEventController *keyboard = gtk_event_controller_key_new();
  g_signal_connect(keyboard, "key-pressed", G_CALLBACK(key_pressed), window);
  g_signal_connect(keyboard, "key-released", G_CALLBACK(key_released), window);
  g_signal_connect(keyboard, "modifiers", G_CALLBACK(modifiers), window);
  gtk_widget_add_controller(window->view, keyboard);
}
