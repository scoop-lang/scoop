#include "window.h"

DWORD sw_style(SWWindow *window) {
  DWORD style = WS_CLIPCHILDREN | WS_CLIPSIBLINGS;
  if (window->fullscreen)
    return style | WS_POPUP;
  style |= WS_SYSMENU | (window->frame_mode != 2 ? WS_CAPTION : WS_POPUP);
  if (window->resizable)
    style |= WS_THICKFRAME;
  if (window->minimize_button)
    style |= WS_MINIMIZEBOX;
  if (window->maximize_button && window->resizable)
    style |= WS_MAXIMIZEBOX;
  return style;
}

SWSnapshot sw_snapshot(SWWindow *window) {
  RECT client = {0}, outer = {0};
  GetClientRect(window->hwnd, &client);
  GetWindowRect(window->hwnd, &outer);
  SWSnapshot result = {0};
  result.scale = GetDpiForWindow(window->hwnd) / 96.0;
  if (result.scale <= 0)
    result.scale = window->scale;
  result.minimized = IsIconic(window->hwnd);
  result.pixel_width = result.minimized ? 0 : client.right;
  result.pixel_height = result.minimized ? 0 : client.bottom;
  result.width = result.pixel_width / result.scale;
  result.height = result.pixel_height / result.scale;
  result.x = outer.left;
  result.y = outer.top;
  result.outer_width = outer.right - outer.left;
  result.outer_height = outer.bottom - outer.top;
  result.visible = IsWindowVisible(window->hwnd);
  DWORD cloaked = 0;
  DwmGetWindowAttribute(window->hwnd, DWMWA_CLOAKED, &cloaked, sizeof(cloaked));
  result.occluded = result.minimized || cloaked != 0;
  result.focused = GetFocus() == window->hwnd;
  result.maximized = IsZoomed(window->hwnd);
  result.fullscreen = window->fullscreen;
  result.dark = sw_theme(window);
  return result;
}

void sw_emit(SWWindow *window, int32_t kind) {
  if (window->destroying)
    return;
  SWEvent *event = sw_event(window, kind);
  if (!event)
    return;
  SWSnapshot snapshot = sw_snapshot(window);
  if (kind == 2)
    sw_frame_geometry(window, snapshot);
  if (kind == 2 || kind == 3)
    sw_keyboard_geometry(window);
  switch (kind) {
  case 2:
    event->data.x = snapshot.width;
    event->data.y = snapshot.height;
    event->data.z = snapshot.pixel_width;
    event->data.w = snapshot.pixel_height;
    event->data.scale = snapshot.scale;
    break;
  case 3:
    event->data.x = snapshot.x;
    event->data.y = snapshot.y;
    break;
  case 4:
    event->data.flags = snapshot.focused;
    break;
  case 5:
    event->data.flags = snapshot.occluded;
    break;
  case 6:
    event->data.flags = snapshot.dark;
    break;
  case 19:
    event->data.flags = window->input_blocked;
    break;
  default:
    break;
  }
  sw_enqueue(window->application, event);
}

LRESULT CALLBACK sw_window_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
  SWWindow *window = (SWWindow *)GetWindowLongPtrW(hwnd, GWLP_USERDATA);
  if (msg == WM_NCCREATE) {
    window = ((CREATESTRUCTW *)lp)->lpCreateParams;
    window->hwnd = hwnd;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (LONG_PTR)window);
  }
  if (!window)
    return DefWindowProcW(hwnd, msg, wp, lp);
  if (msg == sw_taskbar_created_message)
    window->taskbar_ready = true;
  if (window->input_blocked && sw_modal_input_message(msg))
    return 0;
  LRESULT result;
  if (sw_drop_message(window, msg, wp))
    return 0;
  if (!window->destroying &&
      (sw_accessibility_message(window, msg, wp, lp, &result) ||
       sw_frame_message(window, msg, wp, lp, &result) ||
       sw_menu_message(window, msg, wp, lp, &result) ||
       sw_text(window, msg, wp, lp, &result) ||
       sw_input(window, msg, wp, lp, &result)))
    return result;
  switch (msg) {
  case WM_CLOSE:
    if (window->close_button && !window->input_blocked)
      sw_emit(window, 1);
    return 0;
  case WM_QUERYENDSESSION:
    // The hidden application window receives the single process-level intent.
    return FALSE;
  case WM_SIZE:
    sw_text_geometry_changed(window);
    sw_emit(window, 2);
    sw_emit(window, 5);
    return 0;
  case WM_MOVE:
    sw_emit(window, 3);
    return 0;
  case WM_SETFOCUS:
    sw_text_focus(window, true);
    sw_emit(window, 4);
    return 0;
  case WM_KILLFOCUS:
    sw_text_focus(window, false);
    sw_reset_input(window);
    sw_emit(window, 4);
    return 0;
  case WM_SHOWWINDOW:
    sw_wake(window->application);
    break;
  case WM_PAINT: {
    PAINTSTRUCT paint;
    BeginPaint(hwnd, &paint);
    EndPaint(hwnd, &paint);
    sw_emit(window, 15);
    return 0;
  }
  case WM_ERASEBKGND:
    return 1;
  case WM_GETMINMAXINFO:
    if (!window->fullscreen)
      sw_minmax(window, (MINMAXINFO *)lp);
    return 0;
  case WM_SIZING:
    sw_sizing(window, wp, (RECT *)lp);
    return TRUE;
  case WM_DPICHANGED: {
    window->scale = HIWORD(wp) / 96.0;
    sw_menu_dpi(window, HIWORD(wp));
    RECT *rect = (RECT *)lp;
    if (window->fullscreen) {
      int64_t status = sw_fit_fullscreen(window);
      if (status)
        sw_fail(window->application, (DWORD)status);
    } else if (!SetWindowPos(hwnd, NULL, rect->left, rect->top,
                             rect->right - rect->left, rect->bottom - rect->top,
                             SWP_NOZORDER | SWP_NOACTIVATE)) {
      sw_fail(window->application, GetLastError());
    }
    sw_text_geometry_changed(window);
    sw_emit(window, 2);
    return 0;
  }
  case WM_DISPLAYCHANGE:
    if (window->fullscreen && !IsIconic(hwnd)) {
      int64_t status = sw_fit_fullscreen(window);
      if (status)
        sw_fail(window->application, (DWORD)status);
    }
    break;
  case WM_WINDOWPOSCHANGING:
    if (window->bottom) {
      WINDOWPOS *position = (WINDOWPOS *)lp;
      if (!(position->flags & SWP_NOZORDER))
        position->hwndInsertAfter = HWND_BOTTOM;
    }
    break;
  case WM_SETTINGCHANGE:
  case WM_THEMECHANGED:
    sw_emit(window, 6);
    break;
  case WM_NCDESTROY:
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    break;
  default:
    break;
  }
  return DefWindowProcW(hwnd, msg, wp, lp);
}

SWResult scoop_window_create(SWApplication *app, int64_t identity,
                             const uint8_t *title, int64_t length, double width,
                             double height, int32_t flags, SWWindow *parent,
                             int32_t role) {
  SWWindow *window = calloc(1, sizeof(*window));
  if (!window)
    return (SWResult){NULL, ERROR_NOT_ENOUGH_MEMORY};
  window->application = app;
  window->owning_window = parent;
  window->identity = identity;
  window->frame_mode = (flags & 1) ? 0 : 2;
  window->frame_hover = window->frame_pressed = window->frame_button_down = -1;
  window->resizable = (flags & 2) != 0;
  window->transparent = (flags & 4) != 0;
  window->close_button = window->minimize_button = window->maximize_button =
      true;
  sw_input_init(window);
  window->max_width = window->max_height = -1;
  window->theme = -1;
  window->increment_width = window->increment_height = 1;
  window->scale = GetDpiForSystem() / 96.0;
  WCHAR *text = sw_wide(title, length);
  int64_t error = 0;
  if (!text)
    error = GetLastError();
  else {
    window->hwnd = CreateWindowExW(
        role ? WS_EX_TOOLWINDOW : 0, sw_window_class, text, sw_style(window),
        CW_USEDEFAULT, CW_USEDEFAULT, 0, 0, parent ? parent->hwnd : NULL, NULL,
        GetModuleHandleW(NULL), window);
    free(text);
    if (!window->hwnd)
      error = GetLastError();
    else {
      window->scale = GetDpiForWindow(window->hwnd) / 96.0;
      ImmAssociateContext(window->hwnd, NULL);
      error = sw_size(window, width, height);
      if (!error && window->transparent)
        error = sw_frame_margins(window);
    }
  }
  if (!error)
    return (SWResult){window, 0};
  window->destroying = true;
  if (window->hwnd)
    DestroyWindow(window->hwnd);
  sw_input_destroy(window);
  free(window);
  return (SWResult){NULL, error};
}

SWSnapshot scoop_window_snapshot(SWWindow *window) {
  return sw_snapshot(window);
}
void scoop_window_destroy(SWWindow *window) {
  window->destroying = true;
  sw_drag_window_destroy(window);
  sw_drop_window_destroy(window);
  sw_keyboard_destroy(window);
  sw_accessibility_destroy(window);
  sw_text_window_destroy(window);
  DestroyWindow(window->hwnd);
  sw_input_destroy(window);
  free(window->hit_rects);
  free(window);
}
void *scoop_window_hwnd(SWWindow *window) { return window->hwnd; }
void *scoop_window_hinstance(SWWindow *window) {
  return GetModuleHandleW(NULL);
}

int64_t scoop_window_title(SWWindow *window, const uint8_t *title,
                           int64_t length) {
  WCHAR *text = sw_wide(title, length);
  if (!text)
    return GetLastError();
  BOOL ok = SetWindowTextW(window->hwnd, text);
  DWORD error = ok ? 0 : GetLastError();
  free(text);
  return error;
}
int64_t sw_size(SWWindow *window, double width, double height) {
  if (window->fullscreen)
    return 0;
  double pw = ceil(width * window->scale), ph = ceil(height * window->scale);
  if (pw > INT_MAX / 2 || ph > INT_MAX / 2)
    return ERROR_INVALID_PARAMETER;
  RECT rect = {0, 0, (LONG)pw, (LONG)ph};
  sw_adjust_frame_rect(window, &rect, (UINT)round(window->scale * 96));
  if (!SetWindowPos(window->hwnd, NULL, 0, 0, rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE))
    return GetLastError();
  // The first allocation determines the actual native menu wrapping.
  RECT actual, outer;
  GetClientRect(window->hwnd, &actual);
  GetWindowRect(window->hwnd, &outer);
  LONG extra_width = (LONG)pw - actual.right,
       extra_height = (LONG)ph - actual.bottom;
  if ((extra_width || extra_height) &&
      !SetWindowPos(window->hwnd, NULL, 0, 0,
                    outer.right - outer.left + extra_width,
                    outer.bottom - outer.top + extra_height,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE))
    return GetLastError();
  return 0;
}
int64_t scoop_window_size(SWWindow *window, double width, double height) {
  return window->fullscreen ? -1 : sw_size(window, width, height);
}
int64_t scoop_window_visible(SWWindow *window, int32_t visible) {
  ShowWindow(window->hwnd, visible ? SW_SHOWNOACTIVATE : SW_HIDE);
  sw_wake(window->application);
  return 0;
}
int64_t scoop_window_focus(SWWindow *window) {
  // The foreground lock can deny activation; the public result remains
  // Requested.
  SetForegroundWindow(window->hwnd);
  return 0;
}

int32_t scoop_window_windows_taskbar_ready(HWND hwnd) {
  SWWindow *window = (SWWindow *)GetWindowLongPtrW(hwnd, GWLP_USERDATA);
  return window && window->taskbar_ready;
}
