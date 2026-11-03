#include "wayland.h"
#include "x11.h"

int64_t sw_transparency(SWWindow *window, bool transparent) {
  if (!window->configured)
    return 0;
  if (window->application->kind == SW_X11) {
    SXApplication *app = window->application->backend;
    SXWindow *native = window->backend;
    if (transparent)
      return sx_check(window->application,
                      xcb_delete_property_checked(app->connection,
                                                  native->window, app->opaque));
    uint32_t region[] = {0, 0, window->snapshot.pixel_width,
                         window->snapshot.pixel_height};
    return sx_check(window->application,
                    xcb_change_property_checked(
                        app->connection, XCB_PROP_MODE_REPLACE, native->window,
                        app->opaque, XCB_ATOM_CARDINAL, 32, 4, region));
  }
  SLApplication *app = window->application->backend;
  SLWindow *native = window->backend;
  struct wl_region *region = NULL;
  if (!transparent) {
    region = wl_compositor_create_region(app->compositor);
    if (!region)
      return ENOMEM;
    wl_region_add(region, 0, 0, (int)ceil(window->snapshot.width),
                  (int)ceil(window->snapshot.height));
  }
  wl_surface_set_opaque_region(native->surface, region);
  if (region)
    wl_region_destroy(region);
  return 0;
}
