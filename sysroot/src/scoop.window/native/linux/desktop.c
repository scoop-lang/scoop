#include "window.h"
#include <X11/Xatom.h>
#include <gdk/x11/gdkx.h>

void sw_desktop_bounds(SWWindow *window) {
  GdkSurface *surface = sw_surface(window);
  if (!surface || !GDK_IS_X11_SURFACE(surface))
    return;
  GdkDisplay *display = window->application->display;
  Display *native = gdk_x11_display_get_xdisplay(display);
  Window frame = gdk_x11_surface_get_xid(surface), root, parent, *children;
  unsigned int count;
  bool valid = true;
  gdk_x11_display_error_trap_push(display);
  // Reparenting WMs own the outer frame. Query it instead of guessing extents
  // from GtkWidget allocation, which omits the header and native frame.
  for (;;) {
    children = NULL;
    if (!XQueryTree(native, frame, &root, &parent, &children, &count)) {
      valid = false;
      break;
    }
    if (children)
      XFree(children);
    if (parent == None || parent == root)
      break;
    frame = parent;
  }
  XWindowAttributes geometry;
  Window child;
  int x = 0, y = 0;
  if (valid)
    valid = XGetWindowAttributes(native, frame, &geometry) &&
            XTranslateCoordinates(native, frame, root, 0, 0, &x, &y, &child);
  if (gdk_x11_display_error_trap_pop(display))
    valid = false;
  if (!valid) {
    window->snapshot.x = window->snapshot.y = NAN;
    window->snapshot.outer_width = window->snapshot.outer_height = NAN;
    return;
  }
  double scale = gdk_surface_get_scale(surface);
  double left = ((double)x - geometry.border_width) / scale;
  double top = ((double)y - geometry.border_width) / scale;
  bool moved = window->snapshot.x != left || window->snapshot.y != top;
  window->snapshot.x = left;
  window->snapshot.y = top;
  window->snapshot.outer_width =
      (geometry.width + 2.0 * geometry.border_width) / scale;
  window->snapshot.outer_height =
      (geometry.height + 2.0 * geometry.border_width) / scale;
  if (moved)
    sw_emit(window, 3);
}

static bool supports_move(Display *display, Atom move) {
  Atom supported = XInternAtom(display, "_NET_SUPPORTED", True);
  if (supported == None)
    return false;
  Atom type;
  int format;
  unsigned long count, after;
  unsigned char *bytes = NULL;
  bool found = false;
  if (XGetWindowProperty(display, DefaultRootWindow(display), supported, 0,
                         INT32_MAX / 4, False, XA_ATOM, &type, &format, &count,
                         &after, &bytes) == Success &&
      type == XA_ATOM && format == 32)
    for (unsigned long index = 0; index < count; ++index)
      if (((Atom *)bytes)[index] == move) {
        found = true;
        break;
      }
  if (bytes)
    XFree(bytes);
  return found;
}

int64_t scoop_window_position(SWWindow *window, double x, double y) {
  if (window->application->kind != SW_X11)
    return -1;
  GdkSurface *surface = sw_surface(window);
  if (!surface)
    return -2;
  double scale = gdk_surface_get_scale(surface);
  double native_x = round(x * scale), native_y = round(y * scale);
  if (native_x < INT16_MIN || native_y < INT16_MIN || native_x > INT16_MAX ||
      native_y > INT16_MAX)
    return EOVERFLOW;
  Display *display = gdk_x11_display_get_xdisplay(window->application->display);
  Window xid = gdk_x11_surface_get_xid(surface);
  Atom move = XInternAtom(display, "_NET_MOVERESIZE_WINDOW", False);
  if (supports_move(display, move)) {
    XEvent event = {0};
    event.xclient.type = ClientMessage;
    event.xclient.window = xid;
    event.xclient.message_type = move;
    event.xclient.format = 32;
    event.xclient.data.l[0] =
        NorthWestGravity | (1 << 8) | (1 << 9) | (1 << 12);
    event.xclient.data.l[1] = (long)native_x;
    event.xclient.data.l[2] = (long)native_y;
    if (!XSendEvent(display, DefaultRootWindow(display), False,
                    SubstructureRedirectMask | SubstructureNotifyMask, &event))
      return EIO;
  } else {
    XMoveWindow(display, xid, (int)native_x, (int)native_y);
  }
  XFlush(display);
  return 0;
}
