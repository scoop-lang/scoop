#include "window.h"
#include <gdk/x11/gdkx.h>

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
