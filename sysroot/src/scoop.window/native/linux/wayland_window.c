#include "wayland.h"

void sl_window_scale(SWWindow *window) {
  SLWindow *w = window->backend;
  if (!w)
    return;
  int integer = w->preferred_scale > 0 ? w->preferred_scale : 1;
  if (!w->preferred_scale)
    for (SLEnteredOutput *entry = w->outputs; entry; entry = entry->next)
      if (entry->output->scale > integer)
        integer = entry->output->scale;
  double scale =
      w->viewport && w->fractional_scale > 0 ? w->fractional_scale : integer;
  if (w->viewport && w->fractional_scale > 0) {
    wl_surface_set_buffer_scale(w->surface, 1);
    if (window->configured)
      wp_viewport_set_destination(w->viewport, window->snapshot.width,
                                  window->snapshot.height);
  } else
    wl_surface_set_buffer_scale(w->surface, integer);
  if (window->configured)
    sw_set_geometry(window, window->snapshot.width, window->snapshot.height,
                    scale);
  else
    window->snapshot.scale = scale;
}

static void enter(void *data, struct wl_surface *surface,
                  struct wl_output *native) {
  SWWindow *window = data;
  SLWindow *w = window->backend;
  SLApplication *app = window->application->backend;
  for (SLEnteredOutput *entry = w->outputs; entry; entry = entry->next)
    if (entry->output->output == native)
      return;
  for (SLOutput *output = app->outputs; output; output = output->next) {
    if (output->output != native)
      continue;
    SLEnteredOutput *entry = malloc(sizeof(*entry));
    if (!entry) {
      sw_fail(window->application, ENOMEM);
      return;
    }
    *entry = (SLEnteredOutput){output, w->outputs};
    w->outputs = entry;
    sl_window_scale(window);
    return;
  }
}
static void leave(void *data, struct wl_surface *surface,
                  struct wl_output *output) {
  SWWindow *window = data;
  SLWindow *w = window->backend;
  SLEnteredOutput **link = &w->outputs;
  while (*link) {
    SLEnteredOutput *entry = *link;
    if (entry->output->output == output) {
      *link = entry->next;
      free(entry);
      sl_window_scale(window);
      return;
    }
    link = &entry->next;
  }
}
static void preferred_scale(void *data, struct wl_surface *surface,
                            int32_t scale) {
  SWWindow *window = data;
  if (scale > 0) {
    ((SLWindow *)window->backend)->preferred_scale = scale;
    sl_window_scale(window);
  }
}
static void preferred_transform(void *data, struct wl_surface *surface,
                                uint32_t transform) {
  // The renderer uses the legal normal buffer transform on every output.
}
static const struct wl_surface_listener surface_listener = {
    .enter = enter,
    .leave = leave,
    .preferred_buffer_scale = preferred_scale,
    .preferred_buffer_transform = preferred_transform};

static void fractional_scale(void *data, struct wp_fractional_scale_v1 *object,
                             uint32_t scale) {
  SWWindow *window = data;
  if (scale) {
    ((SLWindow *)window->backend)->fractional_scale = scale / 120.0;
    sl_window_scale(window);
  }
}
static const struct wp_fractional_scale_v1_listener fractional_listener = {
    .preferred_scale = fractional_scale};

static void configure(struct libdecor_frame *frame,
                      struct libdecor_configuration *configuration,
                      void *data) {
  SWWindow *window = data;
  SLWindow *w = window->backend;
  int width, height;
  if (!libdecor_configuration_get_content_size(configuration, frame, &width,
                                               &height)) {
    width = (int)fmax(1, ceil(window->requested_width));
    height = (int)fmax(1, ceil(window->requested_height));
  }
  if (width <= 0)
    width = (int)fmax(1, ceil(window->requested_width));
  if (height <= 0)
    height = (int)fmax(1, ceil(window->requested_height));
  enum libdecor_window_state flags = LIBDECOR_WINDOW_STATE_NONE;
  libdecor_configuration_get_window_state(configuration, &flags);
  struct libdecor_state *state = libdecor_state_new(width, height);
  if (!state) {
    sw_fail(window->application, ENOMEM);
    return;
  }
  libdecor_frame_commit(frame, state, configuration);
  libdecor_state_free(state);
  window->snapshot.visible = w->mapped;
  window->snapshot.maximized = (flags & LIBDECOR_WINDOW_STATE_MAXIMIZED) != 0;
  bool occluded = (flags & LIBDECOR_WINDOW_STATE_SUSPENDED) != 0;
  bool changed = window->snapshot.occluded != occluded;
  window->snapshot.occluded = occluded;
  sw_set_geometry(window, width, height, window->snapshot.scale);
  sl_window_scale(window);
  if (changed)
    sw_emit(window, 5);
  sw_emit(window, 15);
}
static void close_requested(struct libdecor_frame *frame, void *data) {
  SWWindow *window = data;
  if (window->close_button)
    sw_emit(window, 1);
}
static void commit(struct libdecor_frame *frame, void *data) {
  SWWindow *window = data;
  SLWindow *w = window->backend;
  if (window->configured && w->mapped)
    wl_surface_commit(w->surface);
}
static void dismiss(struct libdecor_frame *frame, const char *seat,
                    void *data) {
  sw_cancel_pointer(data, 1);
}
static const struct libdecor_frame_interface frame_interface = {
    .configure = configure,
    .close = close_requested,
    .commit = commit,
    .dismiss_popup = dismiss};

static int create_frame(SWWindow *window) {
  SLApplication *app = window->application->backend;
  SLWindow *w = window->backend;
  w->frame = libdecor_decorate(
      app->decor, w->surface,
      (struct libdecor_frame_interface *)&frame_interface, window);
  if (!w->frame)
    return -2;
  libdecor_frame_set_app_id(w->frame, "org.scoop.Scoop");
  libdecor_frame_set_title(w->frame, w->title);
  return sw_wayland_hints(window);
}

int sw_wayland_create(SWWindow *window, const char *title) {
  SLApplication *app = window->application->backend;
  SLWindow *w = calloc(1, sizeof(*w));
  if (!w)
    return ENOMEM;
  window->backend = w;
  w->title = strdup(title);
  w->surface = wl_compositor_create_surface(app->compositor);
  if (!w->title || !w->surface)
    return ENOMEM;
  wl_surface_add_listener(w->surface, &surface_listener, window);
  if (app->viewporter)
    w->viewport = wp_viewporter_get_viewport(app->viewporter, w->surface);
  if (app->fractional && w->viewport) {
    w->fractional = wp_fractional_scale_manager_v1_get_fractional_scale(
        app->fractional, w->surface);
    wp_fractional_scale_v1_add_listener(w->fractional, &fractional_listener,
                                        window);
  }
  return create_frame(window);
}

void sw_wayland_destroy(SWWindow *window) {
  SLApplication *app = window->application->backend;
  SLWindow *w = window->backend;
  if (!w)
    return;
  if (app->keyboard_window == window) {
    app->keyboard_window = NULL;
    app->repeat_deadline = -1;
  }
  if (app->pointer_window == window)
    app->pointer_window = NULL;
  if (app->text_window == window) {
    if (app->text_enabled) {
      zwp_text_input_v3_disable(app->text_input);
      sl_text_commit(window->application);
      app->text_enabled = false;
    }
    app->text_window = NULL;
    sl_reset_text(window->application);
  }
  if (w->frame)
    libdecor_frame_unref(w->frame);
  if (w->fractional)
    wp_fractional_scale_v1_destroy(w->fractional);
  if (w->viewport)
    wp_viewport_destroy(w->viewport);
  if (w->surface)
    wl_surface_destroy(w->surface);
  while (w->outputs) {
    SLEnteredOutput *entry = w->outputs;
    w->outputs = entry->next;
    free(entry);
  }
  free(w->title);
  free(w);
}

int64_t sw_wayland_title(SWWindow *window, const char *title) {
  SLWindow *w = window->backend;
  char *copy = strdup(title);
  if (!copy)
    return ENOMEM;
  if (w->frame)
    libdecor_frame_set_title(w->frame, copy);
  free(w->title);
  w->title = copy;
  return 0;
}

int64_t sw_wayland_size(SWWindow *window, double width, double height) {
  SLWindow *w = window->backend;
  if (window->configured && w->frame) {
    if (!libdecor_frame_is_floating(w->frame))
      return -1;
    int px = fmax(1, ceil(width)), py = fmax(1, ceil(height));
    struct libdecor_state *state = libdecor_state_new(px, py);
    if (!state)
      return ENOMEM;
    libdecor_frame_commit(w->frame, state, NULL);
    libdecor_state_free(state);
    sw_set_geometry(window, px, py, window->snapshot.scale);
    sl_window_scale(window);
  }
  return 0;
}

int64_t sw_wayland_visible(SWWindow *window, bool visible) {
  SLWindow *w = window->backend;
  if (w->mapped == visible)
    return 0;
  if (visible) {
    if (!w->frame) {
      int error = create_frame(window);
      if (error)
        return error;
    }
    w->mapped = true;
    libdecor_frame_map(w->frame);
  } else {
    w->mapped = false;
    window->snapshot.visible = false;
    window->configured = false;
    window->caret_known = false;
    sw_focus(window, false);
    wl_surface_attach(w->surface, NULL, 0, 0);
    wl_surface_commit(w->surface);
    libdecor_frame_unref(w->frame);
    w->frame = NULL;
  }
  return 0;
}

static void activated(void *data, struct xdg_activation_token_v1 *token,
                      const char *value) {
  SLActivation *request = data;
  SLApplication *app = request->application->backend;
  SLActivation **link = &app->activations;
  while (*link != request)
    link = &(*link)->next;
  *link = request->next;
  for (SWWindow *window = request->application->windows; window;
       window = window->next)
    if (window->identity == request->identity) {
      xdg_activation_v1_activate(app->activation, value,
                                 ((SLWindow *)window->backend)->surface);
      break;
    }
  xdg_activation_token_v1_destroy(token);
  free(request);
}
static const struct xdg_activation_token_v1_listener activation_listener = {
    .done = activated};

int64_t sw_wayland_focus(SWWindow *window) {
  SLApplication *app = window->application->backend;
  SLWindow *w = window->backend;
  if (window->snapshot.focused)
    return 0;
  if (!app->activation || !app->seat || !app->serial)
    return -1;
  SLActivation *request = calloc(1, sizeof(*request));
  if (!request)
    return ENOMEM;
  request->application = window->application;
  request->identity = window->identity;
  request->token = xdg_activation_v1_get_activation_token(app->activation);
  if (!request->token) {
    free(request);
    return ENOMEM;
  }
  request->next = app->activations;
  app->activations = request;
  xdg_activation_token_v1_add_listener(request->token, &activation_listener,
                                       request);
  xdg_activation_token_v1_set_serial(request->token, app->serial, app->seat);
  xdg_activation_token_v1_set_surface(request->token, w->surface);
  xdg_activation_token_v1_set_app_id(request->token, "org.scoop.Scoop");
  xdg_activation_token_v1_commit(request->token);
  return 0;
}

int64_t sw_wayland_hints(SWWindow *window) {
  SLWindow *w = window->backend;
  if (!w->frame)
    return 0;
  libdecor_frame_set_visibility(w->frame, window->decorations);
  enum libdecor_capabilities all =
      LIBDECOR_ACTION_MOVE | LIBDECOR_ACTION_RESIZE | LIBDECOR_ACTION_MINIMIZE |
      LIBDECOR_ACTION_FULLSCREEN | LIBDECOR_ACTION_CLOSE;
  enum libdecor_capabilities caps = LIBDECOR_ACTION_MOVE;
  if (window->resizable)
    caps |= LIBDECOR_ACTION_RESIZE;
  if (window->minimize_button)
    caps |= LIBDECOR_ACTION_MINIMIZE;
  if (window->maximize_button)
    caps |= LIBDECOR_ACTION_FULLSCREEN;
  if (window->close_button)
    caps |= LIBDECOR_ACTION_CLOSE;
  libdecor_frame_set_capabilities(w->frame, caps);
  libdecor_frame_unset_capabilities(w->frame, all & ~caps);
  int min_w = window->resizable ? ceil(window->min_width)
                                : fmax(1, ceil(window->requested_width));
  int min_h = window->resizable ? ceil(window->min_height)
                                : fmax(1, ceil(window->requested_height));
  int max_w = window->resizable
                  ? (window->max_width < 0 ? 0 : ceil(window->max_width))
                  : min_w;
  int max_h = window->resizable
                  ? (window->max_height < 0 ? 0 : ceil(window->max_height))
                  : min_h;
  libdecor_frame_set_min_content_size(w->frame, min_w, min_h);
  libdecor_frame_set_max_content_size(w->frame, max_w, max_h);
  return 0;
}
int64_t sw_wayland_minimized(SWWindow *window, bool value) {
  SLWindow *w = window->backend;
  if (!value || !w->mapped)
    return -1;
  libdecor_frame_set_minimized(w->frame);
  return 0;
}
int64_t sw_wayland_maximized(SWWindow *window, bool value) {
  SLWindow *w = window->backend;
  if (!w->frame)
    return -1;
  if (value)
    libdecor_frame_set_maximized(w->frame);
  else
    libdecor_frame_unset_maximized(w->frame);
  return 0;
}

void *sw_wayland_display(SWWindow *window) {
  return ((SLApplication *)window->application->backend)->display;
}
void *sw_wayland_surface(SWWindow *window) {
  return ((SLWindow *)window->backend)->surface;
}
