#include "x11.h"

static uint8_t visual(SXApplication *x, xcb_visualid_t *id) {
  xcb_depth_iterator_t depths = xcb_screen_allowed_depths_iterator(x->screen);
  for (; depths.rem; xcb_depth_next(&depths)) {
    if (depths.data->depth != 32)
      continue;
    xcb_visualtype_iterator_t visuals = xcb_depth_visuals_iterator(depths.data);
    for (; visuals.rem; xcb_visualtype_next(&visuals))
      if (visuals.data->_class == XCB_VISUAL_CLASS_TRUE_COLOR &&
          visuals.data->red_mask == 0xff0000 &&
          visuals.data->green_mask == 0xff00 &&
          visuals.data->blue_mask == 0xff) {
        *id = visuals.data->visual_id;
        return 32;
      }
  }
  *id = x->screen->root_visual;
  return x->screen->root_depth;
}

int sw_x11_create(SWWindow *window, const char *title) {
  SWApplication *app = window->application;
  SXApplication *x = app->backend;
  SXWindow *w = calloc(1, sizeof(*w));
  if (!w)
    return ENOMEM;
  window->backend = w;
  w->depth = visual(x, &w->visual);
  if (window->transparent && w->depth != 32)
    return -1;
  double pw = ceil(window->requested_width * x->scale);
  double ph = ceil(window->requested_height * x->scale);
  if (pw > UINT16_MAX || ph > UINT16_MAX)
    return EOVERFLOW;
  uint16_t width = (uint16_t)fmax(1, pw), height = (uint16_t)fmax(1, ph);
  w->window = xcb_generate_id(x->connection);
  w->colormap = xcb_generate_id(x->connection);
  int error = sx_check(app, xcb_create_colormap_checked(
                                x->connection, XCB_COLORMAP_ALLOC_NONE,
                                w->colormap, x->screen->root, w->visual));
  if (error)
    return error;
  w->colormap_created = true;
  uint32_t values[] = {
      0, 0,
      XCB_EVENT_MASK_EXPOSURE | XCB_EVENT_MASK_STRUCTURE_NOTIFY |
          XCB_EVENT_MASK_FOCUS_CHANGE | XCB_EVENT_MASK_KEY_PRESS |
          XCB_EVENT_MASK_KEY_RELEASE | XCB_EVENT_MASK_BUTTON_PRESS |
          XCB_EVENT_MASK_BUTTON_RELEASE | XCB_EVENT_MASK_POINTER_MOTION |
          XCB_EVENT_MASK_ENTER_WINDOW | XCB_EVENT_MASK_LEAVE_WINDOW |
          XCB_EVENT_MASK_PROPERTY_CHANGE | XCB_EVENT_MASK_VISIBILITY_CHANGE,
      w->colormap};
  error = sx_check(
      app, xcb_create_window_checked(x->connection, w->depth, w->window,
                                     x->screen->root, 0, 0, width, height, 0,
                                     XCB_WINDOW_CLASS_INPUT_OUTPUT, w->visual,
                                     XCB_CW_BACK_PIXEL | XCB_CW_BORDER_PIXEL |
                                         XCB_CW_EVENT_MASK | XCB_CW_COLORMAP,
                                     values));
  if (error)
    return error;
  w->created = true;
  xcb_atom_t protocols[] = {x->delete_window, x->take_focus};
  xcb_change_property(x->connection, XCB_PROP_MODE_REPLACE, w->window,
                      x->protocols, XCB_ATOM_ATOM, 32, 2, protocols);
  uint32_t pid = getpid();
  xcb_change_property(x->connection, XCB_PROP_MODE_REPLACE, w->window, x->pid,
                      XCB_ATOM_CARDINAL, 32, 1, &pid);
  const char instance[] = "scoop\0Scoop\0";
  xcb_change_property(x->connection, XCB_PROP_MODE_REPLACE, w->window,
                      XCB_ATOM_WM_CLASS, XCB_ATOM_STRING, 8,
                      sizeof(instance) - 1, instance);
  error = sw_x11_title(window, title);
  if (error)
    return error;
  window->snapshot.x = window->snapshot.y = 0;
  window->snapshot.outer_width = width;
  window->snapshot.outer_height = height;
  sw_set_geometry(window, width / x->scale, height / x->scale, x->scale);
  sx_create_ic(window);
  return sw_x11_cursor(window);
}

void sw_x11_destroy(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (!w)
    return;
  sx_destroy_ic(window);
  if (w->created)
    xcb_destroy_window(x->connection, w->window);
  if (w->cursor)
    xcb_free_cursor(x->connection, w->cursor);
  if (w->colormap_created)
    xcb_free_colormap(x->connection, w->colormap);
  free(w);
}

int64_t sw_x11_title(SWWindow *window, const char *title) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  size_t length = strlen(title);
  if (length > UINT32_MAX)
    return EOVERFLOW;
  xcb_change_property(x->connection, XCB_PROP_MODE_REPLACE, w->window,
                      XCB_ATOM_WM_NAME, x->utf8, 8, length, title);
  return sx_check(window->application,
                  xcb_change_property_checked(
                      x->connection, XCB_PROP_MODE_REPLACE, w->window, x->name,
                      x->utf8, 8, length, title));
}

int64_t sw_x11_size(SWWindow *window, double width, double height) {
  SXApplication *x = window->application->backend;
  double pw = ceil(width * window->snapshot.scale),
         ph = ceil(height * window->snapshot.scale);
  if (pw > UINT16_MAX || ph > UINT16_MAX)
    return EOVERFLOW;
  uint32_t values[] = {(uint32_t)fmax(1, pw), (uint32_t)fmax(1, ph)};
  return sx_check(window->application,
                  xcb_configure_window_checked(
                      x->connection, ((SXWindow *)window->backend)->window,
                      XCB_CONFIG_WINDOW_WIDTH | XCB_CONFIG_WINDOW_HEIGHT,
                      values));
}

int64_t sw_x11_visible(SWWindow *window, bool visible) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  return sx_check(window->application,
                  visible ? xcb_map_window_checked(x->connection, w->window)
                          : xcb_unmap_window_checked(x->connection, w->window));
}

int64_t sw_x11_focus(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  xcb_client_message_event_t event = {0};
  event.response_type = XCB_CLIENT_MESSAGE;
  event.format = 32;
  event.window = w->window;
  event.type = x->active;
  event.data.data32[0] = 1;
  event.data.data32[1] = XCB_CURRENT_TIME;
  return sx_check(window->application,
                  xcb_send_event_checked(x->connection, false, x->screen->root,
                                         XCB_EVENT_MASK_SUBSTRUCTURE_REDIRECT |
                                             XCB_EVENT_MASK_SUBSTRUCTURE_NOTIFY,
                                         (const char *)&event));
}

void sx_update_position(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  xcb_translate_coordinates_reply_t *position = xcb_translate_coordinates_reply(
      x->connection,
      xcb_translate_coordinates(x->connection, w->window, x->screen->root, 0,
                                0),
      NULL);
  if (!position)
    return;
  double left = 0, right = 0, top = 0, bottom = 0;
  xcb_get_property_reply_t *extents = xcb_get_property_reply(
      x->connection,
      xcb_get_property(x->connection, false, w->window, x->frame_extents,
                       XCB_ATOM_CARDINAL, 0, 4),
      NULL);
  if (extents && extents->format == 32 &&
      xcb_get_property_value_length(extents) == 16) {
    uint32_t *values = xcb_get_property_value(extents);
    left = values[0];
    right = values[1];
    top = values[2];
    bottom = values[3];
  }
  free(extents);
  double next_x = position->dst_x - left, next_y = position->dst_y - top;
  bool changed = window->snapshot.x != next_x || window->snapshot.y != next_y;
  window->snapshot.x = next_x;
  window->snapshot.y = next_y;
  window->snapshot.outer_width = window->snapshot.pixel_width + left + right;
  window->snapshot.outer_height = window->snapshot.pixel_height + top + bottom;
  free(position);
  if (changed)
    sw_emit(window, 3);
}

void sx_update_state(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  xcb_get_property_reply_t *reply = xcb_get_property_reply(
      x->connection,
      xcb_get_property(x->connection, false, w->window, x->net_state,
                       XCB_ATOM_ATOM, 0, UINT32_MAX),
      NULL);
  bool minimized = false, horizontal = false, vertical = false;
  if (reply && reply->format == 32) {
    const xcb_atom_t *atoms = xcb_get_property_value(reply);
    for (int i = 0; i < xcb_get_property_value_length(reply) / 4; ++i) {
      minimized |= atoms[i] == x->hidden;
      horizontal |= atoms[i] == x->maximized_h;
      vertical |= atoms[i] == x->maximized_v;
    }
  }
  free(reply);
  window->snapshot.maximized = horizontal && vertical;
  if (window->snapshot.minimized != minimized) {
    window->snapshot.minimized = minimized;
    window->snapshot.occluded = minimized;
    sw_emit(window, 5);
  }
}

void sx_window_event(SWApplication *app, xcb_generic_event_t *event) {
  uint8_t type = event->response_type & 127;
  xcb_window_t id;
  switch (type) {
  case XCB_CONFIGURE_NOTIFY:
    id = ((xcb_configure_notify_event_t *)event)->window;
    break;
  case XCB_MAP_NOTIFY:
    id = ((xcb_map_notify_event_t *)event)->window;
    break;
  case XCB_UNMAP_NOTIFY:
    id = ((xcb_unmap_notify_event_t *)event)->window;
    break;
  case XCB_DESTROY_NOTIFY:
    id = ((xcb_destroy_notify_event_t *)event)->window;
    break;
  case XCB_CLIENT_MESSAGE:
    id = ((xcb_client_message_event_t *)event)->window;
    break;
  case XCB_PROPERTY_NOTIFY:
    id = ((xcb_property_notify_event_t *)event)->window;
    break;
  case XCB_EXPOSE:
    id = ((xcb_expose_event_t *)event)->window;
    break;
  case XCB_VISIBILITY_NOTIFY:
    id = ((xcb_visibility_notify_event_t *)event)->window;
    break;
  case XCB_FOCUS_IN:
  case XCB_FOCUS_OUT:
    id = ((xcb_focus_in_event_t *)event)->event;
    break;
  case XCB_KEY_PRESS:
  case XCB_KEY_RELEASE:
  case XCB_BUTTON_PRESS:
  case XCB_BUTTON_RELEASE:
  case XCB_MOTION_NOTIFY:
  case XCB_ENTER_NOTIFY:
  case XCB_LEAVE_NOTIFY:
    id = ((xcb_key_press_event_t *)event)->event;
    break;
  default:
    return;
  }
  SWWindow *window = sx_find(app, id);
  if (!window)
    return;
  SXApplication *x = app->backend;
  switch (type) {
  case XCB_CONFIGURE_NOTIFY: {
    xcb_configure_notify_event_t *configure = (void *)event;
    sw_set_geometry(window, configure->width / x->scale,
                    configure->height / x->scale, x->scale);
    sx_update_position(window);
    break;
  }
  case XCB_MAP_NOTIFY:
    window->snapshot.visible = true;
    window->snapshot.occluded = false;
    sx_update_position(window);
    sw_emit(window, 5);
    sw_emit(window, 15);
    break;
  case XCB_UNMAP_NOTIFY:
    window->snapshot.visible = false;
    sw_reset_input(window);
    break;
  case XCB_DESTROY_NOTIFY:
    ((SXWindow *)window->backend)->window = 0;
    ((SXWindow *)window->backend)->created = false;
    sw_emit(window, 1);
    break;
  case XCB_EXPOSE:
    sw_emit(window, 15);
    break;
  case XCB_VISIBILITY_NOTIFY: {
    bool occluded = ((xcb_visibility_notify_event_t *)event)->state ==
                    XCB_VISIBILITY_FULLY_OBSCURED;
    if (window->snapshot.occluded != occluded) {
      window->snapshot.occluded = occluded;
      sw_emit(window, 5);
    }
    break;
  }
  case XCB_FOCUS_IN:
  case XCB_FOCUS_OUT: {
    xcb_focus_in_event_t *focus = (void *)event;
    if (focus->mode != XCB_NOTIFY_MODE_GRAB &&
        focus->mode != XCB_NOTIFY_MODE_UNGRAB &&
        focus->detail != XCB_NOTIFY_DETAIL_INFERIOR) {
      if (type == XCB_FOCUS_IN) {
        xcb_query_keymap_reply_t *keys = xcb_query_keymap_reply(
            x->connection, xcb_query_keymap(x->connection), NULL);
        app->physical_keys_known = keys != NULL;
        memset(app->physical_keys, 0, sizeof(app->physical_keys));
        if (keys) {
          for (unsigned code = 8; code < 256; ++code)
            app->physical_keys[code - 8] =
                (keys->keys[code / 8] & (1u << (code % 8))) != 0;
          free(keys);
        }
      }
      sw_focus(window, type == XCB_FOCUS_IN);
      sw_x11_ime(window);
    }
    break;
  }
  case XCB_PROPERTY_NOTIFY:
    if (((xcb_property_notify_event_t *)event)->atom == x->net_state)
      sx_update_state(window);
    if (((xcb_property_notify_event_t *)event)->atom == x->frame_extents)
      sx_update_position(window);
    break;
  case XCB_CLIENT_MESSAGE: {
    xcb_client_message_event_t *message = (void *)event;
    if (message->type == x->protocols && message->format == 32) {
      if (message->data.data32[0] == x->delete_window && window->close_button)
        sw_emit(window, 1);
      if (message->data.data32[0] == x->take_focus)
        xcb_set_input_focus(x->connection, XCB_INPUT_FOCUS_PARENT, id,
                            message->data.data32[1]);
    }
    break;
  }
  default:
    sx_input_event(window, event);
    break;
  }
}

void *sw_x11_connection(SWWindow *window) {
  return ((SXApplication *)window->application->backend)->connection;
}
uint32_t sw_x11_window(SWWindow *window) {
  return ((SXWindow *)window->backend)->window;
}
uint32_t sw_x11_visual(SWWindow *window) {
  return ((SXWindow *)window->backend)->visual;
}
