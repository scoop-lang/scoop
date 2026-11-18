#include "../window.h"

void sw_input_init(SWWindow *window) {
  window->input.cursor_visible = true;
  window->input.cursor = LoadCursorW(NULL, IDC_ARROW);
}

void sw_input_destroy(SWWindow *window) {
  sw_contacts_clear(&window->input.contacts);
  for (size_t index = 0;
       index < sizeof(window->input.pressed) / sizeof(window->input.pressed[0]);
       ++index)
    free((void *)window->input.pressed[index].text);
}

bool sw_input(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
              LRESULT *result) {
  *result = 0;
  sw_contact_message(window, message, wp);
  switch (message) {
  case WM_KEYDOWN:
  case WM_KEYUP:
  case WM_SYSKEYDOWN:
  case WM_SYSKEYUP:
    sw_key(window, message, wp, lp);
    // DefWindowProc must still implement system shortcuts such as Alt+F4.
    return false;
  default:
    return sw_pointer_message(window, message, wp, lp, result);
  }
}

void sw_reset_input(SWWindow *window) {
  sw_cancel_contacts(window, SWPointerFocusLost);
  sw_cancel_pointer(window, SWPointerFocusLost);
  sw_cancel_composition(window);
  uint64_t locks = sw_modifiers() & 112;
  for (int n = 0; n < 512; ++n) {
    if (!window->input.key_down[n])
      continue;
    SWEvent *event = sw_event(window, SWKeyboard);
    if (event) {
      event->keyboard = window->input.pressed[n];
      event->keyboard.pressed = 0;
      event->keyboard.repeat = 0;
      event->keyboard.synthetic = 1;
      event->keyboard.modifiers = (int64_t)locks;
      window->input.pressed[n].text = NULL;
      sw_enqueue(window->application, event);
    }
    window->input.key_down[n] = false;
  }
  sw_update_modifiers(window, locks);
  window->surrogate = 0;
}
