#include "x11.h"

int64_t sw_x11_hints(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  double scale = window->snapshot.scale;
  double min_w =
      window->resizable ? window->min_width : window->requested_width;
  double min_h =
      window->resizable ? window->min_height : window->requested_height;
  double max_w =
      window->resizable ? window->max_width : window->requested_width;
  double max_h =
      window->resizable ? window->max_height : window->requested_height;
  if (ceil(min_w * scale) > INT_MAX || ceil(min_h * scale) > INT_MAX ||
      ceil(max_w * scale) > INT_MAX || ceil(max_h * scale) > INT_MAX ||
      ceil(window->increment_width * scale) > INT_MAX ||
      ceil(window->increment_height * scale) > INT_MAX)
    return EOVERFLOW;
  uint32_t size[18] = {0};
  size[0] = (1u << 4) | (1u << 6);
  size[5] = (uint32_t)fmax(1, ceil(min_w * scale));
  size[6] = (uint32_t)fmax(1, ceil(min_h * scale));
  if (max_w >= 0 || max_h >= 0) {
    size[0] |= 1u << 5;
    size[7] = max_w < 0 ? INT_MAX : (uint32_t)fmax(1, ceil(max_w * scale));
    size[8] = max_h < 0 ? INT_MAX : (uint32_t)fmax(1, ceil(max_h * scale));
  }
  size[9] = (uint32_t)fmax(1, ceil(window->increment_width * scale));
  size[10] = (uint32_t)fmax(1, ceil(window->increment_height * scale));
  int result = sx_check(window->application,
                        xcb_change_property_checked(
                            x->connection, XCB_PROP_MODE_REPLACE, w->window,
                            x->normal_hints, x->size_hints, 32, 18, size));
  if (result)
    return result;
  uint32_t functions = (1u << 2) | (window->resizable ? 1u << 1 : 0) |
                       (window->minimize_button ? 1u << 3 : 0) |
                       (window->maximize_button ? 1u << 4 : 0) |
                       (window->close_button ? 1u << 5 : 0);
  uint32_t motif[] = {3, functions, window->decorations ? 1u : 0u, 0, 0};
  return sx_check(
      window->application,
      xcb_change_property_checked(x->connection, XCB_PROP_MODE_REPLACE,
                                  w->window, x->motif, x->motif, 32, 5, motif));
}

int64_t sw_x11_position(SWWindow *window, double px, double py) {
  if (px < INT16_MIN || px > INT16_MAX || py < INT16_MIN || py > INT16_MAX)
    return EOVERFLOW;
  SXApplication *x = window->application->backend;
  uint32_t values[] = {(uint32_t)(int32_t)px, (uint32_t)(int32_t)py};
  return sx_check(window->application,
                  xcb_configure_window_checked(
                      x->connection, ((SXWindow *)window->backend)->window,
                      XCB_CONFIG_WINDOW_X | XCB_CONFIG_WINDOW_Y, values));
}

int64_t sx_state_request(SWWindow *window, bool enabled, xcb_atom_t first,
                         xcb_atom_t second) {
  SXApplication *x = window->application->backend;
  xcb_client_message_event_t event = {0};
  event.response_type = XCB_CLIENT_MESSAGE;
  event.format = 32;
  event.window = ((SXWindow *)window->backend)->window;
  event.type = x->net_state;
  event.data.data32[0] = enabled ? 1 : 0;
  event.data.data32[1] = first;
  event.data.data32[2] = second;
  event.data.data32[3] = 1;
  return sx_check(window->application,
                  xcb_send_event_checked(x->connection, false, x->screen->root,
                                         XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT |
                                             XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY,
                                         (const char *)&event));
}

int64_t sw_x11_level(SWWindow *window, int value) {
  SXApplication *x = window->application->backend;
  int64_t error = sx_state_request(window, value > 0, x->above, 0);
  return error ? error : sx_state_request(window, value < 0, x->below, 0);
}

int64_t sw_x11_minimized(SWWindow *window, bool value) {
  SXApplication *x = window->application->backend;
  if (!value)
    return sw_x11_visible(window, true);
  xcb_intern_atom_reply_t *reply = xcb_intern_atom_reply(
      x->connection,
      xcb_intern_atom(x->connection, false, 15, "WM_CHANGE_STATE"), NULL);
  if (!reply)
    return EPIPE;
  xcb_client_message_event_t event = {0};
  event.response_type = XCB_CLIENT_MESSAGE;
  event.format = 32;
  event.window = ((SXWindow *)window->backend)->window;
  event.type = reply->atom;
  event.data.data32[0] = 3;
  free(reply);
  return sx_check(window->application,
                  xcb_send_event_checked(x->connection, false, x->screen->root,
                                         XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT |
                                             XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY,
                                         (const char *)&event));
}
int64_t sw_x11_maximized(SWWindow *window, bool value) {
  SXApplication *x = window->application->backend;
  return sx_state_request(window, value, x->maximized_h, x->maximized_v);
}
int64_t sw_x11_theme(SWWindow *window, int value) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (value < 0)
    return sx_check(
        window->application,
        xcb_delete_property_checked(x->connection, w->window, x->theme));
  const char *theme = value ? "dark" : "light";
  return sx_check(window->application,
                  xcb_change_property_checked(
                      x->connection, XCB_PROP_MODE_REPLACE, w->window, x->theme,
                      x->utf8, 8, strlen(theme), theme));
}
int64_t sw_x11_attention(SWWindow *window, int value) {
  return sx_state_request(
      window, value >= 0,
      ((SXApplication *)window->application->backend)->attention, 0);
}

int64_t sw_x11_cursor(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  const char *names[] = {"default",     "text",      "crosshair",
                         "pointer",     "grab",      "grabbing",
                         "not-allowed", "ew-resize", "ns-resize"};
  xcb_cursor_t cursor;
  if (window->cursor_visible) {
    cursor = xcb_cursor_load_cursor(x->cursors, names[window->cursor_icon]);
    if (!cursor)
      cursor = xcb_cursor_load_cursor(x->cursors, "left_ptr");
  } else {
    xcb_pixmap_t bitmap = xcb_generate_id(x->connection);
    xcb_gcontext_t gc = xcb_generate_id(x->connection);
    xcb_create_pixmap(x->connection, 1, bitmap, x->screen->root, 1, 1);
    uint32_t zero = 0;
    xcb_create_gc(x->connection, gc, bitmap, XCB_GC_FOREGROUND, &zero);
    xcb_rectangle_t rect = {0, 0, 1, 1};
    xcb_poly_fill_rectangle(x->connection, bitmap, gc, 1, &rect);
    cursor = xcb_generate_id(x->connection);
    xcb_create_cursor(x->connection, cursor, bitmap, bitmap, 0, 0, 0, 0, 0, 0,
                      0, 0);
    xcb_free_gc(x->connection, gc);
    xcb_free_pixmap(x->connection, bitmap);
  }
  if (!cursor)
    return -2;
  int error = sx_check(window->application,
                       xcb_change_window_attributes_checked(
                           x->connection, w->window, XCB_CW_CURSOR, &cursor));
  if (error) {
    xcb_free_cursor(x->connection, cursor);
    return error;
  }
  if (w->cursor)
    xcb_free_cursor(x->connection, w->cursor);
  w->cursor = cursor;
  return 0;
}
