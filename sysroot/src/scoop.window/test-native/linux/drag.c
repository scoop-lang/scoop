#include "../../native/linux/window.h"
#include <X11/extensions/XTest.h>
#include <gdk/x11/gdkx.h>

int32_t scoop_test_gtk_drag_pointer(SWWindow *window, int32_t phase) {
  GdkDisplay *gdk = gtk_widget_get_display(window->window);
  if (!GDK_IS_X11_DISPLAY(gdk))
    return 0;
  Display *display = gdk_x11_display_get_xdisplay(gdk);
  if (phase == 2) {
    int result = XTestFakeButtonEvent(display, 1, False, CurrentTime);
    XFlush(display);
    return result != 0;
  }
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(window->window));
  Window native = gdk_x11_surface_get_xid(surface);
  graphene_point_t point = {phase ? 60 : 24, 24}, position;
  if (!gtk_widget_compute_point(window->view, window->window, &point,
                                &position))
    return 0;
  double offset_x, offset_y;
  gtk_native_get_surface_transform(GTK_NATIVE(window->window), &offset_x,
                                   &offset_y);
  double scale = gdk_surface_get_scale(surface);
  int root_x, root_y;
  Window child;
  if (!XTranslateCoordinates(display, native, DefaultRootWindow(display),
                             (int)round((position.x + offset_x) * scale),
                             (int)round((position.y + offset_y) * scale),
                             &root_x, &root_y, &child))
    return 0;
  int result = XTestFakeMotionEvent(display, -1, root_x, root_y, CurrentTime);
  if (!phase)
    result = result && XTestFakeButtonEvent(display, 1, True, CurrentTime);
  XFlush(display);
  return result != 0;
}
