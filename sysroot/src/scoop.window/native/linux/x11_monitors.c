#include "x11.h"

void sx_monitor_change(SWApplication *app) {
  SWEvent *event = calloc(1, sizeof(*event));
  if (!event) {
    sw_fail(app, ENOMEM);
    return;
  }
  event->data.kind = 100;
  sw_enqueue(app, event);
}

static int64_t refresh(SXApplication *x, xcb_randr_output_t output) {
  int64_t result = 0;
  xcb_randr_get_output_info_reply_t *info = xcb_randr_get_output_info_reply(
      x->connection,
      xcb_randr_get_output_info(x->connection, output, XCB_CURRENT_TIME), NULL);
  if (!info)
    return 0;
  xcb_randr_get_crtc_info_reply_t *crtc =
      info->crtc ? xcb_randr_get_crtc_info_reply(
                       x->connection,
                       xcb_randr_get_crtc_info(x->connection, info->crtc,
                                               XCB_CURRENT_TIME),
                       NULL)
                 : NULL;
  xcb_randr_get_screen_resources_current_reply_t *resources =
      xcb_randr_get_screen_resources_current_reply(
          x->connection,
          xcb_randr_get_screen_resources_current(x->connection,
                                                 x->screen->root),
          NULL);
  if (crtc && resources) {
    xcb_randr_mode_info_t *modes =
        xcb_randr_get_screen_resources_current_modes(resources);
    for (int i = 0;
         i < xcb_randr_get_screen_resources_current_modes_length(resources);
         ++i)
      if (modes[i].id == crtc->mode && modes[i].htotal && modes[i].vtotal) {
        double rate = (double)modes[i].dot_clock * 1000 / modes[i].htotal /
                      modes[i].vtotal;
        if (modes[i].mode_flags & XCB_RANDR_MODE_FLAG_INTERLACE)
          rate *= 2;
        if (modes[i].mode_flags & XCB_RANDR_MODE_FLAG_DOUBLE_SCAN)
          rate /= 2;
        result = (int64_t)round(rate);
      }
  }
  free(resources);
  free(crtc);
  free(info);
  return result;
}

SWMonitors *sw_x11_monitors(SWApplication *app) {
  SXApplication *x = app->backend;
  SWMonitors *result = calloc(1, sizeof(*result));
  if (!result)
    return NULL;
  xcb_randr_get_monitors_reply_t *reply =
      x->randr_event
          ? xcb_randr_get_monitors_reply(
                x->connection,
                xcb_randr_get_monitors(x->connection, x->screen->root, true),
                NULL)
          : NULL;
  size_t count = reply ? xcb_randr_get_monitors_monitors_length(reply) : 1;
  result->items = calloc(count ? count : 1, sizeof(*result->items));
  if (!result->items) {
    free(reply);
    free(result);
    return NULL;
  }
  result->count = count;
  if (!reply) {
    result->items[0] =
        (SWMonitorData){.key = x->screen->root,
                        .primary = 1,
                        .width = x->screen->width_in_pixels,
                        .height = x->screen->height_in_pixels,
                        .pixel_width = x->screen->width_in_pixels,
                        .pixel_height = x->screen->height_in_pixels,
                        .scale = x->scale};
    return result;
  }
  xcb_randr_monitor_info_iterator_t monitors =
      xcb_randr_get_monitors_monitors_iterator(reply);
  for (size_t i = 0; monitors.rem;
       ++i, xcb_randr_monitor_info_next(&monitors)) {
    xcb_randr_monitor_info_t *monitor = monitors.data;
    SWMonitorData *out = &result->items[i];
    *out = (SWMonitorData){.key = monitor->name,
                           .primary = monitor->primary,
                           .x = monitor->x,
                           .y = monitor->y,
                           .width = monitor->width,
                           .height = monitor->height,
                           .pixel_width = monitor->width,
                           .pixel_height = monitor->height,
                           .scale = x->scale};
    xcb_get_atom_name_reply_t *name = xcb_get_atom_name_reply(
        x->connection, xcb_get_atom_name(x->connection, monitor->name), NULL);
    if (name) {
      out->name_length = xcb_get_atom_name_name_length(name);
      out->name =
          (uint8_t *)strndup(xcb_get_atom_name_name(name), out->name_length);
      free(name);
    }
    if (monitor->nOutput)
      out->refresh_millihertz =
          refresh(x, *xcb_randr_monitor_info_outputs(monitor));
  }
  free(reply);
  return result;
}

int64_t sw_x11_current_monitor(SWWindow *window) {
  SWMonitors *monitors = sw_x11_monitors(window->application);
  if (!monitors)
    return 0;
  int64_t result = 0;
  double best = 0;
  for (size_t i = 0; i < monitors->count; ++i) {
    SWMonitorData *monitor = &monitors->items[i];
    double width = fmin(window->snapshot.x + window->snapshot.outer_width,
                        monitor->x + monitor->width) -
                   fmax(window->snapshot.x, monitor->x);
    double height = fmin(window->snapshot.y + window->snapshot.outer_height,
                         monitor->y + monitor->height) -
                    fmax(window->snapshot.y, monitor->y);
    double area = fmax(0, width) * fmax(0, height);
    if (area > best) {
      best = area;
      result = monitor->key;
    }
    free((void *)monitor->name);
  }
  free(monitors->items);
  free(monitors);
  return result;
}
