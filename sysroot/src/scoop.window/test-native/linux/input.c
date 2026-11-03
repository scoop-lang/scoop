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
