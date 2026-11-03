#include "x11.h"
#include <locale.h>

int sx_check(SWApplication *app, xcb_void_cookie_t cookie) {
  SXApplication *x = app->backend;
  xcb_generic_error_t *error = xcb_request_check(x->connection, cookie);
  int result = error ? error->error_code : 0;
  free(error);
  return xcb_connection_has_error(x->connection) ? EPIPE : result;
}

static xcb_atom_t atom(SXApplication *x, const char *name) {
  xcb_intern_atom_reply_t *reply = xcb_intern_atom_reply(
      x->connection, xcb_intern_atom(x->connection, false, strlen(name), name),
      NULL);
  xcb_atom_t result = reply ? reply->atom : XCB_ATOM_NONE;
  free(reply);
  return result;
}

static double scale(SXApplication *x) {
  xcb_get_property_reply_t *reply = xcb_get_property_reply(
      x->connection,
      xcb_get_property(x->connection, false, x->screen->root,
                       XCB_ATOM_RESOURCE_MANAGER, XCB_ATOM_STRING, 0,
                       UINT32_MAX),
      NULL);
  double result = 1;
  if (reply) {
    int length = xcb_get_property_value_length(reply);
    char *text = strndup(xcb_get_property_value(reply), length);
    if (text) {
      for (char *line = text; line && *line;) {
        char *next = strchr(line, '\n');
        if (next)
          *next++ = 0;
        if (!strncmp(line, "Xft.dpi:", 8)) {
          locale_t locale = newlocale(LC_NUMERIC_MASK, "C", NULL);
          if (locale) {
            double dpi = strtod_l(line + 8, NULL, locale);
            freelocale(locale);
            if (isfinite(dpi) && dpi > 0)
              result = dpi / 96.0;
          }
        }
        line = next;
      }
      free(text);
    }
    free(reply);
  }
  return result;
}

void sx_keyboard_refresh(SWApplication *app) {
  SXApplication *x = app->backend;
  struct xkb_keymap *map = xkb_x11_keymap_new_from_device(
      app->xkb, x->connection, x->keyboard_device, XKB_KEYMAP_COMPILE_NO_FLAGS);
  struct xkb_state *state = map ? xkb_x11_state_new_from_device(
                                      map, x->connection, x->keyboard_device)
                                : NULL;
  if (!map || !state) {
    xkb_keymap_unref(map);
    xkb_state_unref(state);
    sw_fail(app, EIO);
    return;
  }
  xkb_state_unref(app->key_state);
  xkb_keymap_unref(app->keymap);
  app->keymap = map;
  app->key_state = state;
}

int sw_x11_initialize(SWApplication *app) {
  SXApplication *x = calloc(1, sizeof(*x));
  if (!x)
    return ENOMEM;
  app->backend = x;
  x->connection = xcb_connect(NULL, &x->screen_number);
  if (!x->connection || xcb_connection_has_error(x->connection))
    return -2;
  xcb_screen_iterator_t screens =
      xcb_setup_roots_iterator(xcb_get_setup(x->connection));
  for (int i = 0; i < x->screen_number && screens.rem; ++i)
    xcb_screen_next(&screens);
  if (!screens.rem)
    return EINVAL;
  x->screen = screens.data;
  app->display_fd = xcb_get_file_descriptor(x->connection);
  uint16_t major, minor;
  uint8_t error;
  if (!xkb_x11_setup_xkb_extension(x->connection, XKB_X11_MIN_MAJOR_XKB_VERSION,
                                   XKB_X11_MIN_MINOR_XKB_VERSION,
                                   XKB_X11_SETUP_XKB_EXTENSION_NO_FLAGS, &major,
                                   &minor, &x->xkb_event, &error))
    return -2;
  x->keyboard_device = xkb_x11_get_core_keyboard_device_id(x->connection);
  if (x->keyboard_device < 0)
    return -2;
  sx_keyboard_refresh(app);
  if (app->error)
    return app->error;
  uint16_t events = XCB_XKB_EVENT_TYPE_NEW_KEYBOARD_NOTIFY |
                    XCB_XKB_EVENT_TYPE_MAP_NOTIFY |
                    XCB_XKB_EVENT_TYPE_STATE_NOTIFY;
  xcb_xkb_select_events(
      x->connection, x->keyboard_device, events, 0, events,
      XCB_XKB_MAP_PART_KEY_TYPES | XCB_XKB_MAP_PART_KEY_SYMS |
          XCB_XKB_MAP_PART_MODIFIER_MAP | XCB_XKB_MAP_PART_EXPLICIT_COMPONENTS |
          XCB_XKB_MAP_PART_KEY_ACTIONS | XCB_XKB_MAP_PART_VIRTUAL_MODS |
          XCB_XKB_MAP_PART_VIRTUAL_MOD_MAP,
      UINT16_MAX, NULL);
  xcb_xkb_per_client_flags_reply_t *flags = xcb_xkb_per_client_flags_reply(
      x->connection,
      xcb_xkb_per_client_flags(x->connection, x->keyboard_device,
                               XCB_XKB_PER_CLIENT_FLAG_DETECTABLE_AUTO_REPEAT,
                               XCB_XKB_PER_CLIENT_FLAG_DETECTABLE_AUTO_REPEAT,
                               0, 0, 0),
      NULL);
  free(flags);
  const xcb_query_extension_reply_t *randr =
      xcb_get_extension_data(x->connection, &xcb_randr_id);
  if (randr && randr->present) {
    xcb_randr_query_version_reply_t *version = xcb_randr_query_version_reply(
        x->connection, xcb_randr_query_version(x->connection, 1, 5), NULL);
    if (version &&
        (version->major_version > 1 || version->minor_version >= 5)) {
      x->randr_event = randr->first_event;
      xcb_randr_select_input(x->connection, x->screen->root,
                             XCB_RANDR_NOTIFY_MASK_SCREEN_CHANGE |
                                 XCB_RANDR_NOTIFY_MASK_OUTPUT_CHANGE |
                                 XCB_RANDR_NOTIFY_MASK_CRTC_CHANGE |
                                 XCB_RANDR_NOTIFY_MASK_RESOURCE_CHANGE);
    }
    free(version);
  }
#define ATOM(field, name) x->field = atom(x, name)
  ATOM(protocols, "WM_PROTOCOLS");
  ATOM(delete_window, "WM_DELETE_WINDOW");
  ATOM(take_focus, "WM_TAKE_FOCUS");
  ATOM(utf8, "UTF8_STRING");
  ATOM(name, "_NET_WM_NAME");
  ATOM(pid, "_NET_WM_PID");
  ATOM(wm_state, "WM_STATE");
  ATOM(net_state, "_NET_WM_STATE");
  ATOM(above, "_NET_WM_STATE_ABOVE");
  ATOM(below, "_NET_WM_STATE_BELOW");
  ATOM(maximized_h, "_NET_WM_STATE_MAXIMIZED_HORZ");
  ATOM(maximized_v, "_NET_WM_STATE_MAXIMIZED_VERT");
  ATOM(hidden, "_NET_WM_STATE_HIDDEN");
  ATOM(attention, "_NET_WM_STATE_DEMANDS_ATTENTION");
  ATOM(active, "_NET_ACTIVE_WINDOW");
  ATOM(moveresize, "_NET_MOVERESIZE_WINDOW");
  ATOM(frame_extents, "_NET_FRAME_EXTENTS");
  ATOM(motif, "_MOTIF_WM_HINTS");
  ATOM(normal_hints, "WM_NORMAL_HINTS");
  ATOM(size_hints, "WM_SIZE_HINTS");
  ATOM(wm_hints, "WM_HINTS");
  ATOM(theme, "_GTK_THEME_VARIANT");
  ATOM(clipboard, "CLIPBOARD");
  ATOM(targets, "TARGETS");
  ATOM(text, "text/plain;charset=utf-8");
  ATOM(transfer, "SCOOP_SELECTION");
  ATOM(incr, "INCR");
  ATOM(timestamp, "TIMESTAMP");
#undef ATOM
  if (!x->timestamp)
    return EPIPE;
  x->scale = scale(x);
  uint32_t mask = XCB_EVENT_MASK_PROPERTY_CHANGE;
  xcb_change_window_attributes(x->connection, x->screen->root,
                               XCB_CW_EVENT_MASK, &mask);
  x->helper = xcb_generate_id(x->connection);
  int status = sx_check(
      app, xcb_create_window_checked(
               x->connection, XCB_COPY_FROM_PARENT, x->helper, x->screen->root,
               0, 0, 1, 1, 0, XCB_WINDOW_CLASS_INPUT_OUTPUT,
               x->screen->root_visual, XCB_CW_EVENT_MASK, &mask));
  if (status)
    return status;
  if (xcb_cursor_context_new(x->connection, x->screen, &x->cursors) < 0)
    return ENOMEM;
  sx_initialize_ime(app);
  return 0;
}

void sw_x11_finish(SWApplication *app) {
  SXApplication *x = app->backend;
  if (!x)
    return;
  sx_clipboard_finish(app);
  if (x->im) {
    xcb_xim_close(x->im);
    xcb_xim_destroy(x->im);
  }
  if (x->cursors)
    xcb_cursor_context_free(x->cursors);
  if (x->connection)
    xcb_disconnect(x->connection);
  free(x);
}

int sw_x11_flush(SWApplication *app) {
  SXApplication *x = app->backend;
  xcb_flush(x->connection);
  return xcb_connection_has_error(x->connection) ? EPIPE : 0;
}

int sw_x11_dispatch(SWApplication *app) {
  SXApplication *x = app->backend;
  // A finite native snapshot gives posted work and deadlines an opportunity to
  // run.
  for (int count = 0; count < 256; ++count) {
    xcb_generic_event_t *event = xcb_poll_for_event(x->connection);
    if (!event)
      break;
    uint8_t type = event->response_type & 127;
    if (x->im && xcb_xim_filter_event(x->im, event)) {
      free(event);
      continue;
    }
    if (sx_clipboard_event(app, event)) {
      free(event);
      continue;
    }
    if (type == x->xkb_event) {
      xcb_xkb_state_notify_event_t *state = (void *)event;
      if (state->xkbType == XCB_XKB_STATE_NOTIFY) {
        xkb_state_update_mask(app->key_state, state->baseMods,
                              state->latchedMods, state->lockedMods,
                              state->baseGroup, state->latchedGroup,
                              state->lockedGroup);
        for (SWWindow *window = app->windows; window; window = window->next)
          if (window->snapshot.focused)
            sw_update_modifiers(window);
      } else
        sx_keyboard_refresh(app);
    } else if (x->randr_event &&
               (type == x->randr_event || type == x->randr_event + 1))
      sx_monitor_change(app);
    else if (type == XCB_PROPERTY_NOTIFY &&
             ((xcb_property_notify_event_t *)event)->window ==
                 x->screen->root) {
      if (((xcb_property_notify_event_t *)event)->atom ==
          XCB_ATOM_RESOURCE_MANAGER) {
        x->scale = scale(x);
        for (SWWindow *window = app->windows; window; window = window->next)
          sw_set_geometry(window, window->snapshot.pixel_width / x->scale,
                          window->snapshot.pixel_height / x->scale, x->scale);
        sx_monitor_change(app);
      }
    } else if (type == 0) {
      // Errors from checked public requests are handled synchronously at their
      // boundary.
      app->error = ((xcb_generic_error_t *)event)->error_code;
    } else
      sx_window_event(app, event);
    free(event);
    if (count == 255)
      sw_wake(app);
  }
  if (xcb_connection_has_error(x->connection))
    return EPIPE;
  return app->error;
}

SWWindow *sx_find(SWApplication *app, xcb_window_t id) {
  for (SWWindow *window = app->windows; window; window = window->next)
    if (((SXWindow *)window->backend)->window == id)
      return window;
  return NULL;
}
