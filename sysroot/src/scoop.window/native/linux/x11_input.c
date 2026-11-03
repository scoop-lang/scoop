#include "x11.h"

void sx_input_event(SWWindow *window, xcb_generic_event_t *event) {
  uint8_t type = event->response_type & 127;
  xcb_key_press_event_t *key = (void *)event;
  uint32_t locks = key->state & (XCB_MOD_MASK_LOCK | XCB_MOD_MASK_2);
  xkb_state_update_mask(window->application->key_state,
                        key->state & 255 & ~locks, 0, locks, 0, 0,
                        (key->state >> 13) & 3);
  if (type == XCB_KEY_PRESS || type == XCB_KEY_RELEASE) {
    if (key->detail < 8)
      return;
    window->application->physical_keys[key->detail - 8] = type == XCB_KEY_PRESS;
    if (sx_ime_key(window, key))
      return;
    uint32_t code = key->detail - 8;
    bool down = type == XCB_KEY_PRESS;
    sw_key(window, code, down, down && window->key_down[code], true);
    return;
  }
  window->pointer_x = key->event_x / window->snapshot.scale;
  window->pointer_y = key->event_y / window->snapshot.scale;
  sw_update_modifiers(window);
  switch (type) {
  case XCB_ENTER_NOTIFY:
    sw_pointer(window, 0, 0, 0, 0, 0);
    break;
  case XCB_LEAVE_NOTIFY:
    if (((xcb_leave_notify_event_t *)event)->mode == XCB_NOTIFY_MODE_UNGRAB)
      sw_cancel_pointer(window, 1);
    sw_pointer(window, 1, 0, 0, 0, 0);
    break;
  case XCB_MOTION_NOTIFY:
    sw_pointer(window, 2, 0, 0, 0, 0);
    break;
  case XCB_BUTTON_PRESS:
  case XCB_BUTTON_RELEASE: {
    unsigned button = key->detail;
    if (button >= 4 && button <= 7) {
      if (type == XCB_BUTTON_PRESS)
        sw_pointer(window, 5, 0,
                   button == 6   ? 1
                   : button == 7 ? -1
                                 : 0,
                   button == 4   ? 1
                   : button == 5 ? -1
                                 : 0,
                   0);
      break;
    }
    int mapped = button == 1 ? 0 : button == 3 ? 1 : button == 2 ? 2 : button;
    sw_pointer(window, type == XCB_BUTTON_PRESS ? 3 : 4, mapped, 0, 0, 0);
    break;
  }
  default:
    break;
  }
}
