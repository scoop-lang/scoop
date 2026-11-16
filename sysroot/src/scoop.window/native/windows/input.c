#include "window.h"

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

static void modifiers(SWWindow *window, uint64_t value) {
  if (value == window->modifiers)
    return;
  window->modifiers = value;
  SWEvent *event = sw_event(window, SWModifiers);
  if (event) {
    event->modifiers = (int64_t)value;
    sw_enqueue(window->application, event);
  }
}

static void key(SWWindow *window, UINT msg, WPARAM vk, LPARAM lp) {
  UINT scan = (UINT)((lp >> 16) & 0xff);
  UINT code = scan | ((lp & (1LL << 24)) ? 0x100 : 0);
  bool down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
  uint64_t mods = sw_modifiers();
  modifiers(window, mods);
  if (vk == VK_PROCESSKEY || (down && sw_text_composing(window)))
    return;
  if (!down && !window->key_down[code])
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
    free((void *)window->pressed[code].text);
    window->pressed[code] = *data;
    if (data->text) {
      uint8_t *copy = malloc((size_t)data->text_length + 1);
      if (!copy) {
        sw_free_event(event);
        window->pressed[code].text = NULL;
        sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
        return;
      }
      memcpy(copy, data->text, (size_t)data->text_length + 1);
      window->pressed[code].text = copy;
    }
    window->key_down[code] = true;
  } else {
    *data = window->pressed[code];
    data->pressed = 0;
    data->repeat = 0;
    data->modifiers = (int64_t)mods;
    window->pressed[code].text = NULL;
    window->key_down[code] = false;
  }
  if (down) {
    event->native_input = true;
    GetCursorPos(&event->input_position);
  }
  sw_enqueue(window->application, event);
}

void sw_cancel_pointer(SWWindow *window, int reason) {
  sw_clicks_reset(&window->clicks);
  memset(window->pressed_clicks, 0, sizeof(window->pressed_clicks));
  if (!window->pressed_buttons)
    return;
  window->pressed_buttons = 0;
  SWEvent *event = sw_event(window, SWPointer);
  if (event) {
    event->pointer.kind = SWPointerCancelled;
    event->pointer.cancel_reason = reason;
    sw_enqueue(window->application, event);
  }
  if (GetCapture() == window->hwnd)
    ReleaseCapture();
}

void sw_reset_input(SWWindow *window) {
  sw_cancel_pointer(window, SWPointerFocusLost);
  sw_cancel_composition(window);
  uint64_t locks = sw_modifiers() & 112;
  for (int n = 0; n < 512; ++n) {
    if (!window->key_down[n])
      continue;
    SWEvent *event = sw_event(window, SWKeyboard);
    if (event) {
      event->keyboard = window->pressed[n];
      event->keyboard.pressed = 0;
      event->keyboard.repeat = 0;
      event->keyboard.synthetic = 1;
      event->keyboard.modifiers = (int64_t)locks;
      window->pressed[n].text = NULL;
      sw_enqueue(window->application, event);
    }
    window->key_down[n] = false;
  }
  modifiers(window, locks);
  window->surrogate = 0;
}

static void pointer(SWWindow *window, SWPointerKind kind, POINT position,
                    int32_t button, int clicks, double dx, double dy) {
  SWEvent *event = sw_event(window, SWPointer);
  if (!event)
    return;
  SWPointerData *data = &event->pointer;
  data->kind = kind;
  data->button = button;
  data->click_count = clicks;
  data->modifiers = (int64_t)sw_modifiers();
  data->scale = GetDpiForWindow(window->hwnd) / 96.0;
  data->physical_x = position.x;
  data->physical_y = position.y;
  data->x = position.x / data->scale;
  data->y = position.y / data->scale;
  if (kind == SWPointerLineWheel)
    sw_pointer_scroll(data, false, dx, dy, SWScrollNone, SWScrollNone);
  if (kind == SWPointerPressed) {
    event->native_input = true;
    event->input_position = position;
    ClientToScreen(window->hwnd, &event->input_position);
  }
  sw_enqueue(window->application, event);
}

bool sw_input(SWWindow *window, UINT msg, WPARAM wp, LPARAM lp,
              LRESULT *result) {
  *result = 0;
  POINT position = {GET_X_LPARAM(lp), GET_Y_LPARAM(lp)};
  switch (msg) {
  case WM_CAPTURECHANGED:
    if (window->pressed_buttons)
      sw_cancel_pointer(window, SWPointerCaptureLost);
    return false;
  case WM_CANCELMODE:
    sw_cancel_pointer(window, SWPointerCaptureLost);
    return false;
  case WM_KEYDOWN:
  case WM_KEYUP:
  case WM_SYSKEYDOWN:
  case WM_SYSKEYUP:
    key(window, msg, wp, lp);
    // DefWindowProc must still implement system shortcuts such as Alt+F4.
    return false;
  case WM_SETCURSOR:
    if (LOWORD(lp) == HTCLIENT) {
      SetCursor(window->cursor_visible ? window->cursor : NULL);
      *result = TRUE;
      return true;
    }
    return false;
  case WM_MOUSEMOVE:
    if (window->pressed_buttons && window->clicks.count > 0) {
      POINT screen = position;
      if (!ClientToScreen(window->hwnd, &screen)) {
        sw_fail(window->application, GetLastError());
        return true;
      }
      sw_clicks_move(&window->clicks, screen.x, screen.y,
                     GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG));
    }
    if (!window->pointer_inside) {
      window->pointer_inside = true;
      TRACKMOUSEEVENT track = {.cbSize = sizeof(track),
                               .dwFlags = TME_LEAVE,
                               .hwndTrack = window->hwnd};
      if (!TrackMouseEvent(&track))
        sw_fail(window->application, GetLastError());
      pointer(window, SWPointerEntered, position, 0, 0, 0, 0);
    }
    pointer(window, SWPointerMoved, position, 0, 0, 0, 0);
    return true;
  case WM_MOUSELEAVE:
    window->pointer_inside = false;
    GetCursorPos(&position);
    ScreenToClient(window->hwnd, &position);
    pointer(window, SWPointerLeft, position, 0, 0, 0, 0);
    return true;
  case WM_LBUTTONDOWN:
  case WM_RBUTTONDOWN:
  case WM_MBUTTONDOWN:
  case WM_XBUTTONDOWN:
  case WM_LBUTTONDBLCLK:
  case WM_RBUTTONDBLCLK:
  case WM_MBUTTONDBLCLK:
  case WM_XBUTTONDBLCLK:
  case WM_LBUTTONUP:
  case WM_RBUTTONUP:
  case WM_MBUTTONUP:
  case WM_XBUTTONUP: {
    bool up = msg == WM_LBUTTONUP || msg == WM_RBUTTONUP ||
              msg == WM_MBUTTONUP || msg == WM_XBUTTONUP;
    bool twice = msg == WM_LBUTTONDBLCLK || msg == WM_RBUTTONDBLCLK ||
                 msg == WM_MBUTTONDBLCLK || msg == WM_XBUTTONDBLCLK;
    int button = msg >= WM_XBUTTONDOWN   ? 2 + GET_XBUTTON_WPARAM(wp)
                 : msg >= WM_MBUTTONDOWN ? 2
                 : msg >= WM_RBUTTONDOWN ? 1
                                         : 0;
    if (button >= 32)
      return false;
    uint32_t mask = UINT32_C(1) << button;
    if (up && !(window->pressed_buttons & mask)) {
      *result = TRUE;
      return true;
    }
    if (!up) {
      POINT screen = position;
      if (!ClientToScreen(window->hwnd, &screen)) {
        sw_fail(window->application, GetLastError());
        return true;
      }
      window->pressed_clicks[button] = sw_clicks_press(
          &window->clicks, (uint32_t)GetMessageTime(), button, screen.x,
          screen.y, GetDoubleClickTime(), GetSystemMetrics(SM_CXDOUBLECLK),
          GetSystemMetrics(SM_CYDOUBLECLK), twice);
      window->pressed_buttons |= mask;
      if (GetCapture() != window->hwnd)
        SetCapture(window->hwnd);
    } else
      window->pressed_buttons &= ~mask;
    pointer(window, up ? SWPointerReleased : SWPointerPressed, position, button,
            window->pressed_clicks[button], 0, 0);
    if (up)
      window->pressed_clicks[button] = 0;
    if (!window->pressed_buttons && GetCapture() == window->hwnd)
      ReleaseCapture();
    *result = TRUE;
    return true;
  }
  case WM_MOUSEWHEEL:
  case WM_MOUSEHWHEEL: {
    ScreenToClient(window->hwnd, &position);
    double delta = GET_WHEEL_DELTA_WPARAM(wp) / (double)WHEEL_DELTA;
    pointer(window, SWPointerLineWheel, position, 0, 0,
            msg == WM_MOUSEHWHEEL ? -delta : 0,
            msg == WM_MOUSEWHEEL ? delta : 0);
    return true;
  }
  default:
    return false;
  }
}

int64_t scoop_window_cursor_icon(SWWindow *window, int32_t icon) {
  const WCHAR *names[] = {IDC_ARROW, IDC_IBEAM,  IDC_CROSS,
                          IDC_HAND,  IDC_HAND,   IDC_SIZEALL,
                          IDC_NO,    IDC_SIZEWE, IDC_SIZENS};
  HCURSOR cursor = LoadCursorW(NULL, names[icon]);
  if (!cursor)
    return GetLastError();
  window->cursor = cursor;
  if (window->pointer_inside)
    SetCursor(window->cursor_visible ? cursor : NULL);
  return 0;
}
int64_t scoop_window_cursor_visible(SWWindow *window, int32_t visible) {
  window->cursor_visible = visible != 0;
  if (window->pointer_inside)
    SetCursor(visible ? window->cursor : NULL);
  return 0;
}
