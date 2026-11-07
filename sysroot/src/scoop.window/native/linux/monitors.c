#include "window.h"
#include <X11/Xatom.h>
#include <gdk/x11/gdkx.h>

static void work_area(SWApplication *app, SWMonitorData *value) {
  value->work_x = value->work_y = value->work_width = value->work_height = NAN;
  if (!GDK_IS_X11_DISPLAY(app->display))
    return;
  Display *display = gdk_x11_display_get_xdisplay(app->display);
  Atom current = XInternAtom(display, "_NET_CURRENT_DESKTOP", True);
  Atom work = XInternAtom(display, "_NET_WORKAREA", True);
  if (current == None || work == None)
    return;
  Atom type;
  int format;
  unsigned long count, after;
  unsigned char *bytes = NULL;
  int status = XGetWindowProperty(display, DefaultRootWindow(display), current,
                                  0, 1, False, XA_CARDINAL, &type, &format,
                                  &count, &after, &bytes);
  if (status != Success || type != XA_CARDINAL || format != 32 || count != 1) {
    if (bytes)
      XFree(bytes);
    return;
  }
  unsigned long desktop = ((unsigned long *)bytes)[0];
  XFree(bytes);
  if (desktop > LONG_MAX / 4)
    return;
  bytes = NULL;
  status = XGetWindowProperty(display, DefaultRootWindow(display), work,
                              (long)desktop * 4, 4, False, XA_CARDINAL, &type,
                              &format, &count, &after, &bytes);
  if (status == Success && type == XA_CARDINAL && format == 32 && count == 4) {
    const unsigned long *rect = (const unsigned long *)bytes;
    double x = (int32_t)rect[0] / value->scale;
    double y = (int32_t)rect[1] / value->scale;
    double right = fmin(x + rect[2] / value->scale, value->x + value->width);
    double bottom = fmin(y + rect[3] / value->scale, value->y + value->height);
    x = fmax(x, value->x);
    y = fmax(y, value->y);
    if (right > x && bottom > y) {
      value->work_x = x;
      value->work_y = y;
      value->work_width = right - x;
      value->work_height = bottom - y;
    }
  }
  if (bytes)
    XFree(bytes);
}

static int64_t monitor_key(SWApplication *app, GdkMonitor *monitor) {
  int64_t *key = g_object_get_data(G_OBJECT(monitor), "scoop-monitor-id");
  if (!key) {
    if (app->next_monitor == INT64_MAX) {
      sw_fail(app, EOVERFLOW);
      return 0;
    }
    key = g_new(int64_t, 1);
    *key = ++app->next_monitor;
    g_object_set_data_full(G_OBJECT(monitor), "scoop-monitor-id", key, g_free);
  }
  return *key;
}
void sw_monitors_changed(GListModel *model, guint position, guint removed,
                         guint added, gpointer data) {
  SWApplication *app = data;
  SWEvent *event = calloc(1, sizeof(*event));
  if (!event) {
    sw_fail(app, ENOMEM);
    return;
  }
  event->data.kind = 100;
  sw_enqueue(app, event);
}
void *scoop_window_monitors(void) {
  SWApplication *app = sw_application;
  SWMonitors *result = calloc(1, sizeof(*result));
  if (!result)
    return NULL;
  size_t count = g_list_model_get_n_items(app->monitors);
  result->items = calloc(count ? count : 1, sizeof(*result->items));
  if (!result->items) {
    free(result);
    return NULL;
  }
  for (size_t index = 0; index < count; ++index) {
    GdkMonitor *monitor = g_list_model_get_item(app->monitors, index);
    GdkRectangle geometry;
    gdk_monitor_get_geometry(monitor, &geometry);
    double scale = gdk_monitor_get_scale(monitor);
    const char *label = gdk_monitor_get_model(monitor);
    if (!label)
      label = gdk_monitor_get_connector(monitor);
    if (!label)
      label = "Display";
    char *name = strdup(label);
    if (!name) {
      g_object_unref(monitor);
      for (size_t i = 0; i < result->count; ++i)
        free((void *)result->items[i].name);
      free(result->items);
      free(result);
      return NULL;
    }
    result->items[result->count++] = (SWMonitorData){
        .key = monitor_key(app, monitor),
        .primary = 0,
        .refresh_millihertz = gdk_monitor_get_refresh_rate(monitor),
        .x = geometry.x,
        .y = geometry.y,
        .width = geometry.width,
        .height = geometry.height,
        .pixel_width = ceil(geometry.width * scale),
        .pixel_height = ceil(geometry.height * scale),
        .scale = scale,
        .name = (uint8_t *)name,
        .name_length = (int64_t)strlen(name)};
    work_area(app, &result->items[result->count - 1]);
    g_object_unref(monitor);
  }
  return result;
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
  GdkSurface *surface = sw_surface(window);
  GdkMonitor *monitor = surface ? gdk_display_get_monitor_at_surface(
                                      window->application->display, surface)
                                : NULL;
  return monitor ? monitor_key(window->application, monitor) : 0;
}

typedef struct {
  int64_t error;
  double x, y, width, height;
} SWDesktopRect;

int32_t scoop_window_desktop_capabilities(void) {
  return GDK_IS_X11_DISPLAY(sw_application->display) ? 1 | 2 : 0;
}
SWDesktopRect scoop_window_system_taskbar_bounds(void) {
  return (SWDesktopRect){.error = -1};
}
double scoop_window_system_menu_bar_height(void) { return NAN; }
