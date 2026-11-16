#include "../window.h"

uint64_t sw_modifiers(void) {
  uint64_t value = 0;
  if (GetKeyState(VK_SHIFT) & 0x8000)
    value |= 1;
  if (GetKeyState(VK_CONTROL) & 0x8000)
    value |= 2;
  if (GetKeyState(VK_MENU) & 0x8000)
    value |= 4;
  if ((GetKeyState(VK_LWIN) | GetKeyState(VK_RWIN)) & 0x8000)
    value |= 8;
  if (GetKeyState(VK_CAPITAL) & 1)
    value |= 16;
  if (GetKeyState(VK_NUMLOCK) & 1)
    value |= 32;
  if (GetKeyState(VK_SCROLL) & 1)
    value |= 64;
  const int keys[] = {VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL,
                      VK_LMENU,  VK_RMENU,  VK_LWIN,     VK_RWIN};
  for (int n = 0; n < 8; ++n)
    if (GetKeyState(keys[n]) & 0x8000)
      value |= UINT64_C(256) << n;
  return value;
}

void sw_update_modifiers(SWWindow *window, uint64_t value) {
  if (value == window->input.modifiers)
    return;
  window->input.modifiers = value;
  SWEvent *event = sw_event(window, SWModifiers);
  if (event) {
    event->modifiers = (int64_t)value;
    sw_enqueue(window->application, event);
  }
}

void sw_key(SWWindow *window, UINT msg, WPARAM vk, LPARAM lp) {
  UINT scan = (UINT)((lp >> 16) & 0xff);
  UINT code = scan | ((lp & (1LL << 24)) ? 0x100 : 0);
  bool down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
  uint64_t mods = sw_modifiers();
  sw_update_modifiers(window, mods);
  if (vk == VK_PROCESSKEY || (down && sw_text_composing(window)))
    return;
  if (!down && !window->input.key_down[code])
    return;
  SWEvent *event = sw_event(window, SWKeyboard);
  if (!event)
    return;
  SWKeyboardData *data = &event->keyboard;
  data->physical_code = code;
  data->modifiers = (int64_t)mods;
  data->logical_code = (int64_t)vk;
  data->pressed = down;
  data->repeat = down && (lp & (1LL << 30));
  if (down) {
    BYTE state[256];
    WCHAR text[8];
    GetKeyboardState(state);
    if (!((state[VK_RMENU] & 0x80) && (state[VK_CONTROL] & 0x80)))
      state[VK_CONTROL] = state[VK_LCONTROL] = state[VK_RCONTROL] = 0;
    // Flag 4 preserves the system dead-key state; TranslateMessage owns
    // composition.
    int count =
        ToUnicodeEx((UINT)vk, scan, state, text, 8, 4, GetKeyboardLayout(0));
    if (count < 0) {
      data->dead = 1;
      count = -count;
    }
    if (count > 0 && text[0] >= 0x20)
      data->text = sw_utf8(text, count, &data->text_length);
    free((void *)window->input.pressed[code].text);
    window->input.pressed[code] = *data;
    if (data->text) {
      uint8_t *copy = malloc((size_t)data->text_length + 1);
      if (!copy) {
        sw_free_event(event);
        window->input.pressed[code].text = NULL;
        sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
        return;
      }
      memcpy(copy, data->text, (size_t)data->text_length + 1);
      window->input.pressed[code].text = copy;
    }
    window->input.key_down[code] = true;
  } else {
    *data = window->input.pressed[code];
    data->pressed = 0;
    data->repeat = 0;
    data->modifiers = (int64_t)mods;
    window->input.pressed[code].text = NULL;
    window->input.key_down[code] = false;
  }
  if (down) {
    event->native_input = true;
    GetCursorPos(&event->input_position);
  }
  sw_enqueue(window->application, event);
}
