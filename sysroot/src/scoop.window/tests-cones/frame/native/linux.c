#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <adwaita.h>
#pragma clang diagnostic pop
#include <X11/extensions/XTest.h>
#include <X11/extensions/shape.h>
#include <gdk/x11/gdkx.h>
#include <math.h>
#include <stdint.h>

static GtkWidget *toolbar(GtkWidget *widget) {
  if (ADW_IS_TOOLBAR_VIEW(widget))
    return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *found = toolbar(child);
    if (found)
      return found;
  }
  return NULL;
}

int32_t scoop_test_frame_backend(void) {
  return GDK_IS_X11_DISPLAY(gdk_display_get_default()) ? 1 : 2;
}

double scoop_test_frame_query(GtkWindow *window, GtkWidget *view, int32_t field,
                              double x, double y) {
  static GtkWidget *original;
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(window));
  GdkDisplay *display = gtk_widget_get_display(view);
  switch (field) {
  case 0:
    if (!gtk_window_get_decorated(window))
      return 2;
    return adw_toolbar_view_get_extend_content_to_top_edge(
               ADW_TOOLBAR_VIEW(toolbar(GTK_WIDGET(window))))
               ? 1
               : 0;
  case 1:
    if (!original)
      original = view;
    return original == view;
  case 2:
  case 5: {
    if (!GDK_IS_X11_DISPLAY(display))
      return -1;
    Display *xdisplay = gdk_x11_display_get_xdisplay(display);
    Window xid = gdk_x11_surface_get_xid(surface);
    graphene_point_t point = GRAPHENE_POINT_INIT(x, y), native;
    if (!gtk_widget_compute_point(view, GTK_WIDGET(window), &point, &native))
      return -1;
    double dx, dy;
    gtk_native_get_surface_transform(GTK_NATIVE(window), &dx, &dy);
    int scale = gdk_surface_get_scale_factor(surface);
    int px = (int)round((native.x + dx) * scale),
        py = (int)round((native.y + dy) * scale);
    if (field == 5) {
      int root_x, root_y;
      Window child;
      if (!XTranslateCoordinates(xdisplay, xid, DefaultRootWindow(xdisplay), px,
                                 py, &root_x, &root_y, &child))
        return 0;
      XRaiseWindow(xdisplay, xid);
      XTestFakeMotionEvent(xdisplay, DefaultScreen(xdisplay), root_x, root_y,
                           0);
      int result = XTestFakeButtonEvent(xdisplay, 1, True, 0);
      XTestFakeButtonEvent(xdisplay, 1, False, 0);
      XFlush(xdisplay);
      return result != 0;
    }
    int count = 0, order = 0;
    XRectangle *rects =
        XShapeGetRectangles(xdisplay, xid, ShapeInput, &count, &order);
    int contains = 0;
    for (int i = 0; i < count; ++i)
      if (px >= rects[i].x && py >= rects[i].y &&
          px < rects[i].x + rects[i].width && py < rects[i].y + rects[i].height)
        contains = 1;
    if (rects)
      XFree(rects);
    return contains;
  }
  default:
    return -1;
  }
}
