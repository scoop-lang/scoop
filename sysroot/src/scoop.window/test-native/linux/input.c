#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <xcb/xcb.h>

int32_t scoop_test_x11_input(xcb_connection_t *connection,
                             xcb_window_t window) {
  xcb_get_geometry_reply_t *geometry = xcb_get_geometry_reply(
      connection, xcb_get_geometry(connection, window), NULL);
  if (!geometry)
    return 0;
  xcb_key_press_event_t event = {0};
  event.root = geometry->root;
  free(geometry);
  event.event = window;
  event.event_x = 25;
  event.event_y = 30;
  event.same_screen = 1;
  const uint8_t kinds[] = {XCB_KEY_PRESS, XCB_KEY_RELEASE, XCB_BUTTON_PRESS,
                           XCB_BUTTON_RELEASE};
  const uint32_t masks[] = {
      XCB_EVENT_MASK_KEY_PRESS, XCB_EVENT_MASK_KEY_RELEASE,
      XCB_EVENT_MASK_BUTTON_PRESS, XCB_EVENT_MASK_BUTTON_RELEASE};
  for (unsigned index = 0; index < 4; ++index) {
    event.response_type = kinds[index];
    event.detail = index < 2 ? 38 : 1;
    xcb_generic_error_t *error = xcb_request_check(
        connection, xcb_send_event_checked(connection, false, window,
                                           masks[index], (const char *)&event));
    if (error) {
      free(error);
      return 0;
    }
  }
  return 1;
}

int32_t scoop_test_x11_opaque_region(xcb_connection_t *connection,
                                     xcb_window_t window, int32_t width,
                                     int32_t height) {
  const char name[] = "_NET_WM_OPAQUE_REGION";
  xcb_intern_atom_reply_t *atom = xcb_intern_atom_reply(
      connection, xcb_intern_atom(connection, false, sizeof(name) - 1, name),
      NULL);
  if (!atom)
    return 0;
  xcb_get_property_reply_t *reply = xcb_get_property_reply(
      connection,
      xcb_get_property(connection, false, window, atom->atom, XCB_ATOM_CARDINAL,
                       0, 4),
      NULL);
  free(atom);
  if (!reply)
    return 0;
  bool valid;
  if (!width && !height)
    valid = reply->type == XCB_ATOM_NONE;
  else {
    uint32_t *values = xcb_get_property_value(reply);
    valid = reply->format == 32 && reply->type == XCB_ATOM_CARDINAL &&
            xcb_get_property_value_length(reply) == 16 && !values[0] &&
            !values[1] && values[2] == (uint32_t)width &&
            values[3] == (uint32_t)height;
  }
  free(reply);
  return valid;
}
