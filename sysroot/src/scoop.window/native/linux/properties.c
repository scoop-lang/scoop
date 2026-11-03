#include "x11.h"

static int64_t hints(SWWindow *window) {
  return window->application->kind == SW_X11 ? sw_x11_hints(window)
                                             : sw_wayland_hints(window);
}

int64_t scoop_window_position(SWWindow *window, double x, double y) {
  return window->application->kind == SW_X11 ? sw_x11_position(window, x, y)
                                             : -1;
}

int64_t scoop_window_constraints(SWWindow *window, double min_width,
                                 double min_height, double max_width,
                                 double max_height) {
  if (min_width > INT_MAX || min_height > INT_MAX || max_width > INT_MAX ||
      max_height > INT_MAX)
    return EOVERFLOW;
  window->min_width = min_width;
  window->min_height = min_height;
  window->max_width = max_width;
  window->max_height = max_height;
  int64_t result = hints(window);
  if (result)
    return result;
  double width = fmax(min_width, window->requested_width);
  double height = fmax(min_height, window->requested_height);
  if (max_width >= 0)
    width = fmin(max_width, width);
  if (max_height >= 0)
    height = fmin(max_height, height);
  if (width != window->requested_width || height != window->requested_height) {
    result = window->application->kind == SW_X11
                 ? sw_x11_size(window, width, height)
                 : sw_wayland_size(window, width, height);
    if (!result) {
      window->requested_width = width;
      window->requested_height = height;
    }
  }
  return result;
}

int64_t scoop_window_resize_increments(SWWindow *window, double width,
                                       double height) {
  if (window->application->kind == SW_WAYLAND && (width != 1 || height != 1))
    return -1;
  window->increment_width = width;
  window->increment_height = height;
  return hints(window);
}
int64_t scoop_window_resizable(SWWindow *window, int32_t value) {
  window->resizable = value != 0;
  return hints(window);
}
int64_t scoop_window_decorations(SWWindow *window, int32_t value) {
  window->decorations = value != 0;
  return hints(window);
}
int64_t scoop_window_buttons(SWWindow *window, int32_t close, int32_t minimize,
                             int32_t maximize) {
  window->close_button = close != 0;
  window->minimize_button = minimize != 0;
  window->maximize_button = maximize != 0;
  return hints(window);
}
int64_t scoop_window_level(SWWindow *window, int32_t value) {
  return window->application->kind == SW_X11 ? sw_x11_level(window, value)
                                             : (value ? -1 : 0);
}
int64_t scoop_window_minimized(SWWindow *window, int32_t value) {
  return window->application->kind == SW_X11
             ? sw_x11_minimized(window, value)
             : sw_wayland_minimized(window, value);
}
int64_t scoop_window_maximized(SWWindow *window, int32_t value) {
  return window->application->kind == SW_X11
             ? sw_x11_maximized(window, value)
             : sw_wayland_maximized(window, value);
}
int64_t scoop_window_theme(SWWindow *window, int32_t value) {
  return window->application->kind == SW_X11 ? sw_x11_theme(window, value)
                                             : (value < 0 ? 0 : -1);
}
int64_t scoop_window_transparent(SWWindow *window, int32_t value) {
  if (value && window->application->kind == SW_X11 &&
      ((SXWindow *)window->backend)->depth != 32)
    return -1;
  window->transparent = value != 0;
  return 0;
}
int64_t scoop_window_blur(SWWindow *window, int32_t value) {
  return value ? -1 : 0;
}
int64_t scoop_window_content_protection(SWWindow *window, int32_t value) {
  return value ? -1 : 0;
}
int64_t scoop_window_attention(SWWindow *window, int32_t value) {
  return window->application->kind == SW_X11 ? sw_x11_attention(window, value)
                                             : (value < 0 ? 0 : -1);
}
int64_t scoop_window_cursor_icon(SWWindow *window, int32_t value) {
  window->cursor_icon = value;
  return window->application->kind == SW_X11 ? sw_x11_cursor(window)
                                             : sw_wayland_cursor(window);
}
int64_t scoop_window_cursor_visible(SWWindow *window, int32_t value) {
  window->cursor_visible = value != 0;
  return window->application->kind == SW_X11 ? sw_x11_cursor(window)
                                             : sw_wayland_cursor(window);
}
