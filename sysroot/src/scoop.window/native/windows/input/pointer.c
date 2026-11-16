#include "../window.h"

void sw_cancel_pointer(SWWindow *window, int reason) {
  sw_clicks_reset(&window->input.clicks);
  memset(window->input.pressed_clicks, 0, sizeof(window->input.pressed_clicks));
  if (!window->input.pressed_buttons)
    return;
  window->input.pressed_buttons = 0;
  SWEvent *event = sw_event(window, SWPointer);
  if (event) {
    event->pointer.kind = SWPointerCancelled;
    event->pointer.cancel_reason = reason;
    sw_enqueue(window->application, event);
  }
  if (GetCapture() == window->hwnd)
    ReleaseCapture();
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

bool sw_pointer_message(SWWindow *window, UINT msg, WPARAM wp, LPARAM lp,
                        LRESULT *result) {
  *result = 0;
  // Preserve the native compatibility-message path without duplicating UI
  // input.
  if (sw_compat_mouse_message(msg))
    return false;
  POINT position = {GET_X_LPARAM(lp), GET_Y_LPARAM(lp)};
  switch (msg) {
  case WM_CAPTURECHANGED:
    if (window->input.pressed_buttons)
      sw_cancel_pointer(window, SWPointerCaptureLost);
    return false;
  case WM_CANCELMODE:
    sw_cancel_contacts(window, SWPointerCaptureLost);
    sw_cancel_pointer(window, SWPointerCaptureLost);
    return false;
  case WM_SETCURSOR:
    if (LOWORD(lp) == HTCLIENT) {
      SetCursor(window->input.cursor_visible ? window->input.cursor : NULL);
      *result = TRUE;
      return true;
    }
    return false;
  case WM_MOUSEMOVE:
    if (window->input.pressed_buttons && window->input.clicks.count > 0) {
      POINT screen = position;
      if (!ClientToScreen(window->hwnd, &screen)) {
        sw_fail(window->application, GetLastError());
        return true;
      }
      sw_clicks_move(&window->input.clicks, screen.x, screen.y,
                     GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG));
    }
    if (!window->input.pointer_inside) {
      window->input.pointer_inside = true;
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
    window->input.pointer_inside = false;
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
    if (up && !(window->input.pressed_buttons & mask)) {
      *result = TRUE;
      return true;
    }
    if (!up) {
      POINT screen = position;
      if (!ClientToScreen(window->hwnd, &screen)) {
        sw_fail(window->application, GetLastError());
        return true;
      }
      window->input.pressed_clicks[button] = sw_clicks_press(
          &window->input.clicks, (uint32_t)GetMessageTime(), button, screen.x,
          screen.y, GetDoubleClickTime(), GetSystemMetrics(SM_CXDOUBLECLK),
          GetSystemMetrics(SM_CYDOUBLECLK), twice);
      window->input.pressed_buttons |= mask;
      if (GetCapture() != window->hwnd)
        SetCapture(window->hwnd);
    } else
      window->input.pressed_buttons &= ~mask;
    pointer(window, up ? SWPointerReleased : SWPointerPressed, position, button,
            window->input.pressed_clicks[button], 0, 0);
    if (up)
      window->input.pressed_clicks[button] = 0;
    if (!window->input.pressed_buttons && GetCapture() == window->hwnd)
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
  window->input.cursor = cursor;
  if (window->input.pointer_inside)
    SetCursor(window->input.cursor_visible ? cursor : NULL);
  return 0;
}
int64_t scoop_window_cursor_visible(SWWindow *window, int32_t visible) {
  window->input.cursor_visible = visible != 0;
  if (window->input.pointer_inside)
    SetCursor(visible ? window->input.cursor : NULL);
  return 0;
}
