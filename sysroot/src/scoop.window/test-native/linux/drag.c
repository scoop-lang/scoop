#include "../../native/linux/window.h"
#include <X11/extensions/XTest.h>
#include <gdk/x11/gdkx.h>
#include <string.h>

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

// The X server owns pointer motion. Inspect the actual GTK native surface at
// the current pointer, after it is mapped, without accessing the source's
// internals.
int32_t scoop_test_gtk_drag_icon(SWWindow *window) {
  GdkDisplay *gdk = gtk_widget_get_display(window->window);
  Display *display = gdk_x11_display_get_xdisplay(gdk);
  Window root, parent, *children = NULL;
  unsigned count = 0;
  if (!XQueryTree(display, DefaultRootWindow(display), &root, &parent,
                  &children, &count))
    return 0;
  int result = 0;
  for (unsigned i = 0; i < count; ++i) {
    GdkSurface *surface = gdk_x11_surface_lookup_for_display(gdk, children[i]);
    GtkNative *native = surface ? gtk_native_get_for_surface(surface) : NULL;
    if (!native || !GTK_IS_DRAG_ICON(native) ||
        !gdk_surface_get_mapped(surface))
      continue;
    GtkWidget *child = gtk_drag_icon_get_child(GTK_DRAG_ICON(native));
    if (!GTK_IS_PICTURE(child)) {
      result = -1;
      break;
    }
    GdkPaintable *image = gtk_picture_get_paintable(GTK_PICTURE(child));
    if (!image || gdk_paintable_get_intrinsic_width(image) != 24 ||
        gdk_paintable_get_intrinsic_height(image) != 12) {
      result = -2;
      break;
    }
    Window under;
    int rx, ry, x, y;
    unsigned mask;
    double scale = gdk_surface_get_scale(surface);
    if (!XQueryPointer(display, children[i], &root, &under, &rx, &ry, &x, &y,
                       &mask) ||
        x != lround(5.0 * scale) || y != lround(7.0 * scale))
      continue;
    GtkSnapshot *snapshot = gtk_snapshot_new();
    gdk_paintable_snapshot(image, snapshot, 24, 12);
    GskRenderNode *node = gtk_snapshot_free_to_node(snapshot);
    if (!node) {
      result = -3;
      break;
    }
    cairo_surface_t *pixels =
        cairo_image_surface_create(CAIRO_FORMAT_ARGB32, 24, 12);
    cairo_t *cr = cairo_create(pixels);
    gsk_render_node_draw(node, cr);
    cairo_destroy(cr);
    cairo_surface_flush(pixels);
    uint32_t pixel;
    memcpy(&pixel,
           cairo_image_surface_get_data(pixels) +
               6 * cairo_image_surface_get_stride(pixels) + 12 * 4,
           sizeof(pixel));
    cairo_surface_destroy(pixels);
    gsk_render_node_unref(node);
    result =
        (pixel >> 24) == 128 && (pixel & 255) >= 119 && (pixel & 255) <= 121
            ? 1
            : -4;
    break;
  }
  if (children)
    XFree(children);
  return result;
}
