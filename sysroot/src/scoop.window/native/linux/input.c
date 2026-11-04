#include "window.h"
#include <xkbcommon/xkbcommon-keysyms.h>

uint64_t sw_modifiers(SWApplication *app) {
  if (!app->key_state)
    return 0;
  struct xkb_state *state = app->key_state;
  uint64_t result = SW_ALT_GRAPH_KNOWN;
  if (xkb_state_mod_name_is_active(state, "LevelThree",
                                   XKB_STATE_MODS_EFFECTIVE) > 0 ||
      xkb_state_mod_name_is_active(state, "Mod5", XKB_STATE_MODS_EFFECTIVE) > 0)
    result |= SW_ALT_GRAPH;
  if (xkb_state_mod_name_is_active(state, XKB_MOD_NAME_SHIFT,
                                   XKB_STATE_MODS_EFFECTIVE) > 0)
    result |= SW_SHIFT;
  if (xkb_state_mod_name_is_active(state, XKB_MOD_NAME_CTRL,
                                   XKB_STATE_MODS_EFFECTIVE) > 0)
    result |= SW_CONTROL;
  if (xkb_state_mod_name_is_active(state, XKB_MOD_NAME_ALT,
                                   XKB_STATE_MODS_EFFECTIVE) > 0)
    result |= SW_ALT;
  if (xkb_state_mod_name_is_active(state, XKB_MOD_NAME_LOGO,
                                   XKB_STATE_MODS_EFFECTIVE) > 0)
    result |= SW_SUPER;
  if (xkb_state_led_name_is_active(state, XKB_LED_NAME_CAPS) > 0)
    result |= SW_CAPS;
  if (xkb_state_led_name_is_active(state, XKB_LED_NAME_NUM) > 0)
    result |= SW_NUM;
  if (xkb_state_led_name_is_active(state, XKB_LED_NAME_SCROLL) > 0)
    result |= SW_SCROLL;
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

static bool compose(SWWindow *window, xkb_keysym_t symbol) {
  if (!window->compose)
    return false;
  xkb_compose_state_feed(window->compose, symbol);
  switch (xkb_compose_state_get_status(window->compose)) {
  case XKB_COMPOSE_COMPOSING:
    return true;
  case XKB_COMPOSE_CANCELLED:
    xkb_compose_state_reset(window->compose);
    return true;
  case XKB_COMPOSE_COMPOSED: {
    int size = xkb_compose_state_get_utf8(window->compose, NULL, 0);
    char *text = malloc((size_t)size + 1);
    if (text) {
      xkb_compose_state_get_utf8(window->compose, text, (size_t)size + 1);
      sw_commit(window, text, size);
      free(text);
    } else
      sw_fail(window->application, ENOMEM);
    xkb_compose_state_reset(window->compose);
    return true;
  }
  case XKB_COMPOSE_NOTHING:
    return false;
  }
  return false;
}

void sw_key(SWWindow *window, uint32_t code, bool down, bool repeat,
            bool text) {
  SWApplication *app = window->application;
  if (!app->key_state)
    return;
  bool tracked = code < sizeof(window->key_down) / sizeof(window->key_down[0]);
  if (!down && (!tracked || !window->key_down[code]))
    return;
  sw_update_modifiers(window);
  xkb_keysym_t symbol = xkb_state_key_get_one_sym(app->key_state, code + 8);
  bool shortcut = (window->modifiers & SW_SUPER) != 0 ||
                  (!(window->modifiers & SW_ALT_GRAPH) &&
                   (window->modifiers & (SW_CONTROL | SW_ALT)) != 0);
  if (down && text && !shortcut && compose(window, symbol))
    return;
  if (down && window->preedit_active &&
      (symbol == XKB_KEY_Return || symbol == XKB_KEY_Escape))
    return;
  SWEvent *event = sw_event(window, 10);
  if (!event)
    return;
  event->data.a = code;
  event->data.b = window->modifiers;
  event->data.c = symbol;
  event->data.flags = (down ? 1 : 0) | (repeat ? 2 : 0);
  if (symbol >= XKB_KEY_dead_grave && symbol <= XKB_KEY_dead_greek)
    event->data.flags |= 8;
  char value[64];
  int size = xkb_keysym_to_utf8(symbol, value, sizeof(value));
  if (size > 1 && (uint8_t)value[0] >= 32 && (uint8_t)value[0] != 127) {
    event->data.text = (uint8_t *)strndup(value, size - 1);
    if (!event->data.text) {
      sw_free_event(event);
      sw_fail(app, ENOMEM);
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
      sw_fail(app, ENOMEM);
      return;
    }
    window->key_down[code] = down;
  }
  sw_enqueue(app, event);
  if (down && text && !shortcut && size > 1 && (uint8_t)value[0] >= 32 &&
      (uint8_t)value[0] != 127)
    sw_commit(window, value, size - 1);
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
  }
  window->snapshot.focused = focused;
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
