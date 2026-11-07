#include "tray.h"

static const WCHAR tray_class[] = L"Scoop.Tray.Host";
enum { ST_CALLBACK = WM_APP + 1 };

int64_t st_error(void) {
  DWORD error = GetLastError();
  return error ? (int64_t)error : ERROR_GEN_FAILURE;
}

static void emit(STTray *tray, STEvent event) {
  tray->callback(tray->context, &event);
}

void st_available(STTray *tray, bool available) {
  if (tray->available == available)
    return;
  tray->available = available;
  if (tray->started && !tray->closing)
    emit(tray, (STEvent){.kind = 1, .action = available});
}

static int64_t register_icon(STTray *tray) {
  NOTIFYICONDATAW data = {.cbSize = sizeof(data),
                          .hWnd = tray->hwnd,
                          .uID = 1,
                          .uFlags =
                              NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
                          .uCallbackMessage = ST_CALLBACK,
                          .hIcon = tray->icon};
  memcpy(data.szTip, tray->tooltip, sizeof(tray->tooltip));
  if (!Shell_NotifyIconW(NIM_ADD, &data)) {
    st_available(tray, false);
    return 0;
  }
  data.uVersion = NOTIFYICON_VERSION_4;
  if (!Shell_NotifyIconW(NIM_SETVERSION, &data)) {
    int64_t error = st_error();
    Shell_NotifyIconW(NIM_DELETE, &data);
    return error;
  }
  st_available(tray, true);
  return 0;
}

static void activation(STTray *tray, WPARAM wp, LPARAM lp) {
  UINT message = LOWORD(lp);
  if (HIWORD(lp) != 1 || tray->closing || !tray->available)
    return;
  STEvent event = {.kind = 2};
  if (message == NIN_SELECT || message == NIN_KEYSELECT) {
    event.action = 1;
    event.source = message == NIN_KEYSELECT ? 2 : 1;
    event.button = message == NIN_SELECT ? 1 : 0;
  } else if (message == WM_CONTEXTMENU) {
    event.action = 2;
    event.source = GET_X_LPARAM(wp) == -1 && GET_Y_LPARAM(wp) == -1 ? 2 : 0;
    if (tray->menu)
      event.kind = 5;
  } else if (message == WM_MBUTTONUP) {
    event.action = 3;
    event.source = 1;
    event.button = 3;
  } else {
    return;
  }
  STGeometry rectangle = scoop_tray_geometry(tray);
  if (!rectangle.error) {
    event.anchor = 1;
    event.x = rectangle.x;
    event.y = rectangle.y;
    event.width = rectangle.width;
    event.height = rectangle.height;
    tray->anchor =
        (POINT){(LONG)rectangle.x, (LONG)(rectangle.y + rectangle.height)};
    tray->has_anchor = true;
  } else if (GET_X_LPARAM(wp) != -1 || GET_Y_LPARAM(wp) != -1) {
    event.anchor = 2;
    event.x = GET_X_LPARAM(wp);
    event.y = GET_Y_LPARAM(wp);
    tray->anchor = (POINT){(LONG)event.x, (LONG)event.y};
    tray->has_anchor = true;
  }
  emit(tray, event);
}

static LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wp,
                                    LPARAM lp) {
  STTray *tray = (STTray *)GetWindowLongPtrW(hwnd, GWLP_USERDATA);
  if (message == WM_NCCREATE) {
    tray = ((CREATESTRUCTW *)lp)->lpCreateParams;
    tray->hwnd = hwnd;
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (LONG_PTR)tray);
  }
  if (!tray)
    return DefWindowProcW(hwnd, message, wp, lp);
  if (message == tray->taskbar_created && tray->started && !tray->closing) {
    st_available(tray, false);
    tray->has_anchor = false;
    int64_t error = st_refresh_image(tray);
    if (!error)
      error = register_icon(tray);
    if (error)
      emit(tray, (STEvent){.kind = 6, .error = error});
    return 0;
  }
  switch (message) {
  case ST_CALLBACK:
    activation(tray, wp, lp);
    return 0;
  case WM_DPICHANGED:
  case WM_DISPLAYCHANGE:
  case WM_SETTINGCHANGE:
    if (tray->started && !tray->closing && tray->images) {
      int64_t error = st_refresh_image(tray);
      if (error && error != -2)
        emit(tray, (STEvent){.kind = 6, .error = error});
    }
    return 0;
  case WM_CLOSE:
    return 0;
  case WM_NCDESTROY:
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    break;
  default:
    break;
  }
  return DefWindowProcW(hwnd, message, wp, lp);
}

STResult scoop_tray_create(STCallback callback, void *context,
                           int64_t identity) {
  STTray *tray = calloc(1, sizeof(*tray));
  if (!tray)
    return (STResult){.error = ERROR_NOT_ENOUGH_MEMORY};
  tray->callback = callback;
  tray->context = context;
  tray->taskbar_created = RegisterWindowMessageW(L"TaskbarCreated");
  HINSTANCE instance = GetModuleHandleW(NULL);
  WNDCLASSEXW descriptor = {.cbSize = sizeof(descriptor),
                            .lpfnWndProc = window_proc,
                            .hInstance = instance,
                            .lpszClassName = tray_class};
  if (!tray->taskbar_created ||
      (!RegisterClassExW(&descriptor) &&
       GetLastError() != ERROR_CLASS_ALREADY_EXISTS)) {
    int64_t error = st_error();
    free(tray);
    return (STResult){.error = error};
  }
  // A hidden top-level window receives Explorer's TaskbarCreated broadcast.
  HWND hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, tray_class, L"", WS_POPUP, 0, 0,
                              0, 0, NULL, NULL, instance, tray);
  if (!hwnd) {
    int64_t error = st_error();
    free(tray);
    return (STResult){.error = error};
  }
  return (STResult){tray, 0};
}

int64_t scoop_tray_start(void *pointer) {
  STTray *tray = pointer;
  tray->started = true;
  return register_icon(tray);
}

int32_t scoop_tray_available(void *pointer) {
  return ((STTray *)pointer)->available;
}
int32_t scoop_tray_capabilities(void *pointer) {
  return 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256;
}
int32_t scoop_tray_busy(void *pointer) {
  return ((STTray *)pointer)->presenting;
}

STGeometry scoop_tray_geometry(void *pointer) {
  STTray *tray = pointer;
  if (!tray->available || tray->closing)
    return (STGeometry){.error = -2};
  NOTIFYICONIDENTIFIER identifier = {
      .cbSize = sizeof(identifier), .hWnd = tray->hwnd, .uID = 1};
  RECT bounds;
  if (FAILED(Shell_NotifyIconGetRect(&identifier, &bounds)) ||
      IsRectEmpty(&bounds))
    return (STGeometry){.error = -2};
  return (STGeometry){.x = bounds.left,
                      .y = bounds.top,
                      .width = bounds.right - bounds.left,
                      .height = bounds.bottom - bounds.top};
}

int64_t scoop_tray_tooltip(void *pointer, const uint8_t *text, int64_t length) {
  STTray *tray = pointer;
  if (length < 0 || length > INT_MAX)
    return ERROR_INVALID_PARAMETER;
  int count =
      length ? MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                                   (const char *)text, (int)length, NULL, 0)
             : 0;
  if (length && !count)
    return st_error();
  if (count >= 128)
    return -1;
  WCHAR tooltip[128] = {0};
  if (count &&
      !MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, (const char *)text,
                           (int)length, tooltip, count))
    return st_error();
  if (tray->available) {
    NOTIFYICONDATAW data = {.cbSize = sizeof(data),
                            .hWnd = tray->hwnd,
                            .uID = 1,
                            .uFlags = NIF_TIP | NIF_SHOWTIP};
    memcpy(data.szTip, tooltip, sizeof(tooltip));
    if (!Shell_NotifyIconW(NIM_MODIFY, &data)) {
      st_available(tray, false);
      return -2;
    }
  }
  memcpy(tray->tooltip, tooltip, sizeof(tooltip));
  return 0;
}

int64_t scoop_tray_menu(void *pointer, void *menu, void *actions) {
  ((STTray *)pointer)->menu = menu;
  return 0;
}

STSelection scoop_tray_show_menu(void *pointer) {
  STTray *tray = pointer;
  if (!tray->menu || !tray->available)
    return (STSelection){.error = -2};
  STGeometry geometry = scoop_tray_geometry(tray);
  POINT position;
  TPMPARAMS parameters = {.cbSize = sizeof(parameters)};
  if (!geometry.error) {
    position = (POINT){(LONG)geometry.x, (LONG)(geometry.y + geometry.height)};
    parameters.rcExclude = (RECT){(LONG)geometry.x, (LONG)geometry.y,
                                  (LONG)(geometry.x + geometry.width),
                                  (LONG)(geometry.y + geometry.height)};
  } else if (tray->has_anchor) {
    position = tray->anchor;
  } else {
    return (STSelection){.error = -2};
  }
  tray->has_anchor = false;
  tray->presenting = true;
  SetForegroundWindow(tray->hwnd);
  SetLastError(0);
  UINT command = (UINT)TrackPopupMenuEx(
      tray->menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_WORKAREA,
      position.x, position.y, tray->hwnd, geometry.error ? NULL : &parameters);
  DWORD error = command ? 0 : GetLastError();
  PostMessageW(tray->hwnd, WM_NULL, 0, 0);
  tray->presenting = false;
  return (STSelection){.command = (int32_t)command, .error = error};
}

void scoop_tray_cancel(void *pointer) {
  if (((STTray *)pointer)->presenting)
    EndMenu();
}

void scoop_tray_request_close(void *pointer) {
  STTray *tray = pointer;
  tray->closing = true;
  scoop_tray_cancel(tray);
  NOTIFYICONDATAW data = {.cbSize = sizeof(data), .hWnd = tray->hwnd, .uID = 1};
  if (tray->started)
    Shell_NotifyIconW(NIM_DELETE, &data);
  tray->available = false;
}

void scoop_tray_destroy(void *pointer) {
  STTray *tray = pointer;
  DestroyWindow(tray->hwnd);
  if (tray->icon)
    DestroyIcon(tray->icon);
  st_free_images(tray->images);
  st_free_images(tray->pending_images);
  free(tray);
}
