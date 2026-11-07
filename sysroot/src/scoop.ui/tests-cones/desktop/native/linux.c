#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gdk/x11/gdkx.h>
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <X11/extensions/shape.h>
#include <math.h>
#include <stdint.h>

int32_t scoop_test_ui_region(GtkWindow *window, GtkWidget *view, int32_t field,
                             double x, double y) {
  GdkDisplay *display = gtk_widget_get_display(view);
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(window));
  if (field != 1 || !surface || !GDK_IS_X11_DISPLAY(display))
    return -1;
  graphene_point_t point = GRAPHENE_POINT_INIT(x, y), native;
  if (!gtk_widget_compute_point(view, GTK_WIDGET(window), &point, &native))
    return -1;
  double dx, dy;
  gtk_native_get_surface_transform(GTK_NATIVE(window), &dx, &dy);
  int scale = gdk_surface_get_scale_factor(surface);
  int px = (int)round((native.x + dx) * scale);
  int py = (int)round((native.y + dy) * scale);
  Display *xdisplay = gdk_x11_display_get_xdisplay(display);
  int count = 0, order = 0;
  XRectangle *rects = XShapeGetRectangles(
      xdisplay, gdk_x11_surface_get_xid(surface), ShapeInput, &count, &order);
  int result = 0;
  for (int index = 0; index < count; ++index)
    if (px >= rects[index].x && py >= rects[index].y &&
        px < rects[index].x + rects[index].width &&
        py < rects[index].y + rects[index].height)
      result = 1;
  if (rects)
    XFree(rects);
  return result;
}
