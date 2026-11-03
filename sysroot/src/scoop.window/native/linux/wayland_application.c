#include "wayland.h"

static void output_geometry(void *data, struct wl_output *output, int32_t x,
                            int32_t y, int32_t width, int32_t height,
                            int32_t subpixel, const char *make,
                            const char *model, int32_t transform) {
  SLOutput *value = data;
  if (!value->description)
    value->description = strdup(model);
}
static void output_mode(void *data, struct wl_output *output, uint32_t flags,
                        int32_t width, int32_t height, int32_t refresh) {
  SLOutput *value = data;
  if (flags & WL_OUTPUT_MODE_CURRENT) {
    value->width = width;
    value->height = height;
    value->refresh = refresh;
  }
}
static void output_done(void *data, struct wl_output *output) {
  SLOutput *value = data;
  sl_monitor_changed(value->application);
  for (SWWindow *window = value->application->windows; window;
       window = window->next)
    sl_window_scale(window);
}
static void output_scale(void *data, struct wl_output *output, int32_t factor) {
  if (factor > 0)
    ((SLOutput *)data)->scale = factor;
}
static void output_name(void *data, struct wl_output *output,
                        const char *name) {
  SLOutput *value = data;
  char *copy = strdup(name);
  if (copy) {
    free(value->description);
    value->description = copy;
  }
}
static const struct wl_output_listener output_listener = {
    .geometry = output_geometry,
    .mode = output_mode,
    .done = output_done,
    .scale = output_scale,
    .name = output_name,
    .description = output_name};

static void seat_name(void *data, struct wl_seat *seat, const char *name) {}
static const struct wl_seat_listener seat_listener = {
    .capabilities = sl_seat_capabilities, .name = seat_name};

static void global(void *data, struct wl_registry *registry, uint32_t name,
                   const char *interface, uint32_t version) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  if (!strcmp(interface, wl_compositor_interface.name) && version >= 3)
    w->compositor = wl_registry_bind(registry, name, &wl_compositor_interface,
                                     version < 6 ? version : 6);
  else if (!strcmp(interface, wl_shm_interface.name))
    w->shm = wl_registry_bind(registry, name, &wl_shm_interface, 1);
  else if (!strcmp(interface, wl_seat_interface.name) && !w->seat) {
    w->seat = wl_registry_bind(registry, name, &wl_seat_interface,
                               version < 8 ? version : 8);
    w->seat_name = name;
    wl_seat_add_listener(w->seat, &seat_listener, app);
  } else if (!strcmp(interface, wl_output_interface.name)) {
    SLOutput *output = calloc(1, sizeof(*output));
    if (!output) {
      sw_fail(app, ENOMEM);
      return;
    }
    output->application = app;
    output->name = name;
    output->scale = 1;
    output->output = wl_registry_bind(registry, name, &wl_output_interface,
                                      version < 4 ? version : 4);
    output->next = w->outputs;
    w->outputs = output;
    wl_output_add_listener(output->output, &output_listener, output);
  } else if (!strcmp(interface, wp_viewporter_interface.name))
    w->viewporter =
        wl_registry_bind(registry, name, &wp_viewporter_interface, 1);
  else if (!strcmp(interface, wp_fractional_scale_manager_v1_interface.name))
    w->fractional = wl_registry_bind(
        registry, name, &wp_fractional_scale_manager_v1_interface, 1);
  else if (!strcmp(interface, xdg_activation_v1_interface.name))
    w->activation =
        wl_registry_bind(registry, name, &xdg_activation_v1_interface, 1);
  else if (!strcmp(interface, wl_data_device_manager_interface.name))
    w->data_manager =
        wl_registry_bind(registry, name, &wl_data_device_manager_interface,
                         version < 3 ? version : 3);
  else if (!strcmp(interface, zwp_text_input_manager_v3_interface.name))
    w->text_manager = wl_registry_bind(registry, name,
                                       &zwp_text_input_manager_v3_interface, 1);
}

static void global_remove(void *data, struct wl_registry *registry,
                          uint32_t name) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  SLOutput **link = &w->outputs;
  while (*link) {
    SLOutput *output = *link;
    if (output->name != name) {
      link = &output->next;
      continue;
    }
    *link = output->next;
    for (SWWindow *window = app->windows; window; window = window->next) {
      SLWindow *native = window->backend;
      SLEnteredOutput **entered = &native->outputs;
      while (*entered) {
        if ((*entered)->output == output) {
          SLEnteredOutput *old = *entered;
          *entered = old->next;
          free(old);
        } else
          entered = &(*entered)->next;
      }
      sl_window_scale(window);
    }
    wl_output_destroy(output->output);
    free(output->description);
    free(output);
    sl_monitor_changed(app);
    return;
  }
  if (name == w->seat_name) {
    sl_seat_capabilities(app, w->seat, 0);
    if (w->text_input) {
      zwp_text_input_v3_destroy(w->text_input);
      w->text_input = NULL;
    }
    sl_reset_text(app);
    w->text_window = NULL;
    w->text_enabled = w->text_blocked = w->text_pending = false;
    w->text_serial = 0;
    w->text_connection = 0;
    w->text_revision = -1;
    while (w->text_states) {
      SLTextState *state = w->text_states;
      w->text_states = state->next;
      free(state);
    }
    w->text_tail = NULL;
    if (w->data_device) {
      wl_data_device_release(w->data_device);
      w->data_device = NULL;
    }
    wl_seat_release(w->seat);
    w->seat = NULL;
    w->serial = w->pointer_serial = 0;
  }
}
static const struct wl_registry_listener registry_listener = {
    .global = global, .global_remove = global_remove};

static void decor_error(struct libdecor *context, enum libdecor_error error,
                        const char *message) {
  if (sw_application)
    sw_fail(sw_application, EPROTO);
}
static struct libdecor_interface decor_interface = {.error = decor_error};

int sw_wayland_initialize(SWApplication *app) {
  SLApplication *w = calloc(1, sizeof(*w));
  if (!w)
    return ENOMEM;
  app->backend = w;
  w->repeat_deadline = -1;
  w->text_revision = -1;
  w->display = wl_display_connect(NULL);
  if (!w->display)
    return -2;
  app->display_fd = wl_display_get_fd(w->display);
  w->registry = wl_display_get_registry(w->display);
  wl_registry_add_listener(w->registry, &registry_listener, app);
  if (wl_display_roundtrip(w->display) < 0 ||
      wl_display_roundtrip(w->display) < 0)
    return EPIPE;
  if (!w->compositor || !w->shm)
    return -2;
  w->decor = libdecor_new(w->display, &decor_interface);
  if (!w->decor)
    return -2;
  app->decor_fd = libdecor_get_fd(w->decor);
  if (app->decor_fd < 0)
    return -2;
  w->cursor_surface = wl_compositor_create_surface(w->compositor);
  sl_setup_clipboard(app);
  sl_setup_text(app);
  return app->error;
}

int sw_wayland_dispatch(SWApplication *app) {
  SLApplication *w = app->backend;
  if (libdecor_dispatch(w->decor, 0) < 0 ||
      wl_display_dispatch_pending(w->display) < 0)
    return EPIPE;
  sl_clipboard_flush(app);
  return app->error;
}
int sw_wayland_flush(SWApplication *app) {
  SLApplication *w = app->backend;
  app->display_write_pending = false;
  if (wl_display_flush(w->display) < 0) {
    if (errno != EAGAIN)
      return EPIPE;
    app->display_write_pending = true;
  }
  return 0;
}

void sw_wayland_finish(SWApplication *app) {
  SLApplication *w = app->backend;
  if (!w)
    return;
  sl_finish_clipboard(app);
  sl_reset_text(app);
  while (w->text_states) {
    SLTextState *state = w->text_states;
    w->text_states = state->next;
    free(state);
  }
  while (w->activations) {
    SLActivation *request = w->activations;
    w->activations = request->next;
    xdg_activation_token_v1_destroy(request->token);
    free(request);
  }
  if (w->text_input)
    zwp_text_input_v3_destroy(w->text_input);
  if (w->text_manager)
    zwp_text_input_manager_v3_destroy(w->text_manager);
  if (w->keyboard)
    wl_keyboard_release(w->keyboard);
  if (w->pointer)
    wl_pointer_release(w->pointer);
  if (w->seat)
    wl_seat_release(w->seat);
  if (w->cursor_surface)
    wl_surface_destroy(w->cursor_surface);
  if (w->cursor_theme)
    wl_cursor_theme_destroy(w->cursor_theme);
  if (w->fractional)
    wp_fractional_scale_manager_v1_destroy(w->fractional);
  if (w->viewporter)
    wp_viewporter_destroy(w->viewporter);
  if (w->activation)
    xdg_activation_v1_destroy(w->activation);
  while (w->outputs) {
    SLOutput *output = w->outputs;
    w->outputs = output->next;
    wl_output_destroy(output->output);
    free(output->description);
    free(output);
  }
  if (w->decor)
    libdecor_unref(w->decor);
  if (w->shm)
    wl_shm_destroy(w->shm);
  if (w->compositor)
    wl_compositor_destroy(w->compositor);
  if (w->registry)
    wl_registry_destroy(w->registry);
  if (w->display)
    wl_display_disconnect(w->display);
  free(w);
}

SWWindow *sl_find(SWApplication *app, struct wl_surface *surface) {
  for (SWWindow *window = app->windows; window; window = window->next)
    if (((SLWindow *)window->backend)->surface == surface)
      return window;
  return NULL;
}

void sl_monitor_changed(SWApplication *app) {
  SWEvent *event = calloc(1, sizeof(*event));
  if (!event) {
    sw_fail(app, ENOMEM);
    return;
  }
  event->data.kind = 100;
  sw_enqueue(app, event);
}

SWMonitors *sw_wayland_monitors(SWApplication *app) {
  SLApplication *w = app->backend;
  size_t count = 0;
  for (SLOutput *output = w->outputs; output; output = output->next)
    if (output->width > 0 && output->height > 0)
      ++count;
  SWMonitors *result = calloc(1, sizeof(*result));
  if (!result)
    return NULL;
  result->items = calloc(count ? count : 1, sizeof(*result->items));
  if (!result->items) {
    free(result);
    return NULL;
  }
  for (SLOutput *output = w->outputs; output; output = output->next) {
    if (output->width <= 0 || output->height <= 0)
      continue;
    SWMonitorData *item = &result->items[result->count++];
    *item = (SWMonitorData){.key = output->name,
                            .refresh_millihertz = output->refresh,
                            .x = NAN,
                            .y = NAN,
                            .width = output->width / (double)output->scale,
                            .height = output->height / (double)output->scale,
                            .pixel_width = output->width,
                            .pixel_height = output->height,
                            .scale = output->scale};
    if (output->description) {
      item->name = (uint8_t *)strdup(output->description);
      item->name_length = strlen(output->description);
    }
  }
  return result;
}
int64_t sw_wayland_current_monitor(SWWindow *window) {
  SLWindow *w = window->backend;
  SLOutput *selected = NULL;
  for (SLEnteredOutput *entry = w->outputs; entry; entry = entry->next)
    if (!selected || entry->output->scale > selected->scale)
      selected = entry->output;
  return selected ? selected->name : 0;
}
