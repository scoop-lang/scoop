#include "window.h"

SWResult scoop_window_create(SWApplication *app, int64_t identity,
                             const uint8_t *title, int64_t length, double width,
                             double height, int32_t flags) {
  if (app->error)
    return (SWResult){NULL, -3};
  SWWindow *window = calloc(1, sizeof(*window));
  char *name = strndup((const char *)title, length);
  if (!window || !name) {
    free(window);
    free(name);
    return (SWResult){NULL, ENOMEM};
  }
  window->application = app;
  window->identity = identity;
  window->requested_width = width;
  window->requested_height = height;
  window->max_width = window->max_height = -1;
  window->increment_width = window->increment_height = 1;
  window->decorations = (flags & 1) != 0;
  window->resizable = (flags & 2) != 0;
  window->transparent = (flags & 4) != 0;
  window->close_button = window->minimize_button = window->maximize_button =
      true;
  window->cursor_visible = true;
  window->snapshot.scale = 1;
  window->snapshot.x = window->snapshot.y = NAN;
  window->snapshot.outer_width = window->snapshot.outer_height = NAN;
  window->snapshot.dark = -1;
  window->snapshot.minimized = app->kind == SW_WAYLAND ? -1 : 0;
  window->document.revision = -1;
  if (app->compose_table)
    window->compose =
        xkb_compose_state_new(app->compose_table, XKB_COMPOSE_STATE_NO_FLAGS);
  window->next = app->windows;
  app->windows = window;
  int error = app->kind == SW_X11 ? sw_x11_create(window, name)
                                  : sw_wayland_create(window, name);
  free(name);
  if (error) {
    app->windows = window->next;
    if (app->kind == SW_X11)
      sw_x11_destroy(window);
    else
      sw_wayland_destroy(window);
    xkb_compose_state_unref(window->compose);
    free(window);
    return (SWResult){NULL, error};
  }
  return (SWResult){window, 0};
}

SWSnapshot scoop_window_snapshot(SWWindow *window) { return window->snapshot; }
int32_t scoop_window_configured(SWWindow *window) { return window->configured; }

void scoop_window_destroy(SWWindow *window) {
  SWApplication *app = window->application;
  SWWindow **link = &app->windows;
  while (*link && *link != window)
    link = &(*link)->next;
  if (*link)
    *link = window->next;
  if (app->kind == SW_X11)
    sw_x11_destroy(window);
  else
    sw_wayland_destroy(window);
  for (size_t i = 0; i < sizeof(window->pressed) / sizeof(window->pressed[0]);
       ++i)
    free((void *)window->pressed[i].text);
  xkb_compose_state_unref(window->compose);
  free((void *)window->document.text);
  free(window->preedit);
  free(window);
}

int64_t scoop_window_title(SWWindow *window, const uint8_t *text,
                           int64_t length) {
  char *title = strndup((const char *)text, length);
  if (!title)
    return ENOMEM;
  int64_t result = window->application->kind == SW_X11
                       ? sw_x11_title(window, title)
                       : sw_wayland_title(window, title);
  free(title);
  return result;
}

int64_t scoop_window_size(SWWindow *window, double width, double height) {
  double scale = window->snapshot.scale;
  if (ceil(width * scale) > INT_MAX || ceil(height * scale) > INT_MAX)
    return EOVERFLOW;
  int64_t result = window->application->kind == SW_X11
                       ? sw_x11_size(window, width, height)
                       : sw_wayland_size(window, width, height);
  if (!result) {
    window->requested_width = width;
    window->requested_height = height;
  }
  return result;
}

int64_t scoop_window_visible(SWWindow *window, int32_t visible) {
  int64_t result = window->application->kind == SW_X11
                       ? sw_x11_visible(window, visible)
                       : sw_wayland_visible(window, visible);
  if (!result)
    window->requested_visible = visible != 0;
  sw_wake(window->application);
  return result;
}

int64_t scoop_window_focus(SWWindow *window) {
  return window->application->kind == SW_X11 ? sw_x11_focus(window)
                                             : sw_wayland_focus(window);
}

void *scoop_window_xcb_connection(SWWindow *window) {
  return sw_x11_connection(window);
}
uint32_t scoop_window_xcb_window(SWWindow *window) {
  return sw_x11_window(window);
}
uint32_t scoop_window_xcb_visual(SWWindow *window) {
  return sw_x11_visual(window);
}
void *scoop_window_wl_display(SWWindow *window) {
  return sw_wayland_display(window);
}
void *scoop_window_wl_surface(SWWindow *window) {
  return sw_wayland_surface(window);
}

void *scoop_window_monitors(void) {
  SWApplication *app = sw_application;
  return app->kind == SW_X11 ? sw_x11_monitors(app) : sw_wayland_monitors(app);
}
int64_t scoop_window_monitors_count(SWMonitors *monitors) {
  return monitors->count;
}
SWMonitorData scoop_window_monitor_data(SWMonitors *monitors, int64_t index) {
  return monitors->items[index];
}
void scoop_window_monitors_release(SWMonitors *monitors) {
  for (size_t i = 0; i < monitors->count; ++i)
    free((void *)monitors->items[i].name);
  free(monitors->items);
  free(monitors);
}
int64_t scoop_window_current_monitor(SWWindow *window) {
  return window->application->kind == SW_X11
             ? sw_x11_current_monitor(window)
             : sw_wayland_current_monitor(window);
}
