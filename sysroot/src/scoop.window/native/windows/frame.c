#include "window.h"

int32_t scoop_window_frame_capabilities(void) {
  return SW_FRAME_CLIENT | SW_FRAME_EXTENDED | SW_FRAME_SYSTEM_RESIZE |
         SW_FRAME_MOVE | SW_FRAME_RESIZE | SW_FRAME_MENU | SW_FRAME_REGION |
         SW_FRAME_RESIZE_HIT | SW_FRAME_BUTTON_HIT;
}

void sw_frame_invalidate(SWWindow *window) {
  window->frame_revision += 1;
  free(window->hit_rects);
  window->hit_rects = NULL;
  window->hit_count = 0;
  sw_frame_clear_buttons(window);
  if (window->window_region) {
    window->window_region = false;
    SetWindowRgn(window->hwnd, NULL, TRUE);
  }
  sw_emit(window, 17);
}

uint64_t scoop_window_frame_revision(SWWindow *window) {
  return window->frame_revision;
}

void sw_frame_geometry(SWWindow *window, SWSnapshot snapshot) {
  if (snapshot.width == window->frame_width &&
      snapshot.height == window->frame_height &&
      snapshot.scale == window->frame_scale)
    return;
  window->frame_width = snapshot.width;
  window->frame_height = snapshot.height;
  window->frame_scale = snapshot.scale;
  sw_frame_invalidate(window);
}

int64_t sw_frame_margins(SWWindow *window) {
  MARGINS margins = {0};
  if (window->transparent)
    margins = (MARGINS){-1, -1, -1, -1};
  else if (window->frame_mode == 1) {
    UINT dpi = GetDpiForWindow(window->hwnd);
    margins.cyTopHeight = GetSystemMetricsForDpi(SM_CYCAPTION, dpi) +
                          GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) +
                          GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi);
  }
  HRESULT status = DwmExtendFrameIntoClientArea(window->hwnd, &margins);
  return SUCCEEDED(status) ? 0 : (uint32_t)status;
}

int64_t scoop_window_frame(SWWindow *window, int32_t mode) {
  if (window->frame_mode == mode)
    return 0;
  HMENU menu = GetMenu(window->hwnd);
  if (mode == 1 && menu && GetMenuItemCount(menu) > 0)
    return -1;
  int previous = window->frame_mode;
  window->frame_mode = mode;
  int64_t status = sw_restyle(window);
  if (!status)
    status = sw_frame_margins(window);
  if (status) {
    window->frame_mode = previous;
    sw_restyle(window);
    sw_frame_margins(window);
    return status;
  }
  sw_frame_invalidate(window);
  sw_emit(window, 2);
  return 0;
}

static SWFrameRect screen_rect(SWWindow *window, RECT rect) {
  POINT point = {rect.left, rect.top};
  if (!ScreenToClient(window->hwnd, &point))
    return sw_frame_unknown();
  double scale = GetDpiForWindow(window->hwnd) / 96.0;
  return (SWFrameRect){point.x / scale, point.y / scale,
                       (rect.right - rect.left) / scale,
                       (rect.bottom - rect.top) / scale};
}

SWFrameRect scoop_window_frame_button(SWWindow *window, int32_t role) {
  if (window->frame_mode == 2)
    return sw_frame_unknown();
  TITLEBARINFOEX info = {0};
  info.cbSize = sizeof(info);
  SendMessageW(window->hwnd, WM_GETTITLEBARINFOEX, 0, (LPARAM)&info);
  int index = role == 0 ? 5 : role == 1 ? 2 : 3;
  RECT rect = info.rgrect[index];
  if ((info.rgstate[index] & 0x8000) || rect.right <= rect.left ||
      rect.bottom <= rect.top)
    return sw_frame_unknown();
  return screen_rect(window, rect);
}

SWFrameSnapshot scoop_window_frame_snapshot(SWWindow *window) {
  SWSnapshot size = sw_snapshot(window);
  SWFrameRect layout = {0, 0, size.width, size.height};
  if (window->frame_mode == 1) {
    TITLEBARINFOEX info = {0};
    info.cbSize = sizeof(info);
    SendMessageW(window->hwnd, WM_GETTITLEBARINFOEX, 0, (LPARAM)&info);
    double bottom = NAN;
    if (info.rcTitleBar.bottom > info.rcTitleBar.top) {
      SWFrameRect title = screen_rect(window, info.rcTitleBar);
      bottom = title.y + title.height;
    }
    // NCCALCSIZE can collapse rcTitleBar to the client edge while the native
    // caption buttons still occupy part of the extended client area.
    for (int index = 2; index <= 5; ++index) {
      RECT rect = info.rgrect[index];
      if (!(info.rgstate[index] & STATE_SYSTEM_INVISIBLE) &&
          rect.right > rect.left && rect.bottom > rect.top) {
        SWFrameRect button = screen_rect(window, rect);
        bottom = fmax(bottom, button.y + button.height);
      }
    }
    if (isfinite(bottom)) {
      layout.y = fmin(size.height, fmax(0, bottom));
      layout.height = fmax(0, size.height - layout.y);
    } else
      layout = sw_frame_unknown();
  }
  UINT border = 0;
  double thickness = SUCCEEDED(DwmGetWindowAttribute(window->hwnd, 37, &border,
                                                     sizeof(border)))
                         ? border / size.scale
                         : NAN;
  return (SWFrameSnapshot){window->frame_mode, 1, layout, thickness};
}

int64_t scoop_window_shadow(SWWindow *window, int32_t enabled) {
  return enabled ? 0 : -1;
}

int64_t scoop_window_ignore_mouse(SWWindow *window, int32_t enabled) {
  return enabled ? -1 : 0;
}

int64_t scoop_window_region(SWWindow *window, int32_t kind, int32_t present,
                            const SWFrameRect *rects, int64_t count) {
  if (kind != 0)
    return present ? -1 : 0;
  HRGN region = NULL;
  if (present) {
    region = CreateRectRgn(0, 0, 0, 0);
    if (!region)
      return ERROR_NOT_ENOUGH_MEMORY;
    RECT frame;
    POINT content = {0, 0};
    GetWindowRect(window->hwnd, &frame);
    ClientToScreen(window->hwnd, &content);
    double dx = content.x - frame.left, dy = content.y - frame.top;
    double scale = GetDpiForWindow(window->hwnd) / 96.0;
    for (int64_t i = 0; i < count; ++i) {
      if (rects[i].width == 0 || rects[i].height == 0)
        continue;
      double left = floor(rects[i].x * scale + dx),
             top = floor(rects[i].y * scale + dy);
      double right = ceil((rects[i].x + rects[i].width) * scale + dx);
      double bottom = ceil((rects[i].y + rects[i].height) * scale + dy);
      // GDI regions use signed 27-bit coordinates.
      if (!sw_frame_rect_valid(rects[i]) || left < -67108864 ||
          top < -67108864 || right > 67108863 || bottom > 67108863) {
        DeleteObject(region);
        return ERROR_INVALID_PARAMETER;
      }
      HRGN part = CreateRectRgn((int)left, (int)top, (int)right, (int)bottom);
      if (!part) {
        DeleteObject(region);
        return ERROR_NOT_ENOUGH_MEMORY;
      }
      int result = CombineRgn(region, region, part, RGN_OR);
      DeleteObject(part);
      if (result == ERROR) {
        DeleteObject(region);
        return ERROR_NOT_ENOUGH_MEMORY;
      }
    }
  }
  if (!SetWindowRgn(window->hwnd, region, TRUE)) {
    if (region)
      DeleteObject(region);
    return ERROR_INVALID_PARAMETER;
  }
  window->window_region = present != 0;
  return 0;
}

int64_t scoop_window_hit_regions(SWWindow *window, const SWHitRect *rects,
                                 int64_t count) {
  if (count < 0 || (uint64_t)count > SIZE_MAX / sizeof(*rects))
    return ERROR_INVALID_PARAMETER;
  bool unchanged = (size_t)count == window->hit_count;
  for (int64_t i = 0; i < count; ++i) {
    if (!sw_frame_rect_valid(rects[i].rect) || rects[i].role < 0 ||
        rects[i].role > 12)
      return ERROR_INVALID_PARAMETER;
    if (unchanged) {
      SWHitRect previous = window->hit_rects[i], next = rects[i];
      unchanged = previous.role == next.role &&
                  previous.rect.x == next.rect.x &&
                  previous.rect.y == next.rect.y &&
                  previous.rect.width == next.rect.width &&
                  previous.rect.height == next.rect.height;
    }
  }
  if (unchanged)
    return 0;
  SWHitRect *copy = count ? malloc((size_t)count * sizeof(*copy)) : NULL;
  if (count && !copy)
    return ERROR_NOT_ENOUGH_MEMORY;
  if (count)
    memcpy(copy, rects, (size_t)count * sizeof(*copy));
  free(window->hit_rects);
  window->hit_rects = copy;
  window->hit_count = (size_t)count;
  sw_frame_clear_buttons(window);
  return 0;
}

bool sw_frame_message(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
                      LRESULT *result) {
  if (message == WM_NCCALCSIZE && window->frame_mode != 0) {
    RECT *rect = wp ? &((NCCALCSIZE_PARAMS *)lp)->rgrc[0] : (RECT *)lp;
    RECT outer = *rect;
    DefWindowProcW(window->hwnd, message, wp, lp);
    RECT frame = {0};
    AdjustWindowRectExForDpi(&frame, sw_style(window), FALSE, 0,
                             GetDpiForWindow(window->hwnd));
    LONG menu = rect->top - outer.top + frame.top;
    *rect = outer;
    rect->top += (menu > 0 ? menu : 0);
    if (IsZoomed(window->hwnd)) {
      MONITORINFO monitor = {.cbSize = sizeof(monitor)};
      if (GetMonitorInfoW(
              MonitorFromWindow(window->hwnd, MONITOR_DEFAULTTONEAREST),
              &monitor)) {
        if (rect->left < monitor.rcWork.left)
          rect->left = monitor.rcWork.left;
        if (rect->top < monitor.rcWork.top)
          rect->top = monitor.rcWork.top;
        if (rect->right > monitor.rcWork.right)
          rect->right = monitor.rcWork.right;
        if (rect->bottom > monitor.rcWork.bottom)
          rect->bottom = monitor.rcWork.bottom;
      }
    }
    *result = 0;
    return true;
  }
  return sw_frame_input(window, message, wp, lp, result);
}

void sw_adjust_frame_rect(SWWindow *window, RECT *rect, UINT dpi) {
  if (window->frame_mode == 0) {
    AdjustWindowRectExForDpi(rect, sw_style(window),
                             GetMenu(window->hwnd) != NULL, 0, dpi);
    return;
  }
  if (GetMenu(window->hwnd)) {
    MENUBARINFO info = {.cbSize = sizeof(info)};
    if (GetMenuBarInfo(window->hwnd, OBJID_MENU, 0, &info))
      rect->top -= info.rcBar.bottom - info.rcBar.top;
  }
}

void scoop_window_frame_retire(SWWindow *window) {
  free(window->hit_rects);
  window->hit_rects = NULL;
  window->hit_count = 0;
  sw_frame_clear_buttons(window);
}
