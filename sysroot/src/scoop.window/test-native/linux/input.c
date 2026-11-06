#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <X11/extensions/XTest.h>
#include <gdk/x11/gdkx.h>
#include <stdbool.h>
#include <stdint.h>

bool scoop_test_gtk_x11(void *window) {
  return GDK_IS_X11_DISPLAY(gtk_widget_get_display(GTK_WIDGET(window)));
}
bool scoop_test_gtk_composited(void *window) {
  return gdk_display_is_composited(gtk_widget_get_display(GTK_WIDGET(window)));
}
int32_t scoop_test_gtk_input(void *window, void *view) {
  GdkDisplay *gdk = gtk_widget_get_display(GTK_WIDGET(window));
  if (!GDK_IS_X11_DISPLAY(gdk))
    return 0;
  Display *display = gdk_x11_display_get_xdisplay(gdk);
  Window native =
      gdk_x11_surface_get_xid(gtk_native_get_surface(GTK_NATIVE(window)));
  graphene_point_t point = {24, 24}, position;
  if (!gtk_widget_compute_point(GTK_WIDGET(view), GTK_WIDGET(window), &point,
                                &position))
    return 0;
  double offset_x, offset_y;
  gtk_native_get_surface_transform(GTK_NATIVE(window), &offset_x, &offset_y);
  int root_x, root_y;
  Window child;
  if (!XTranslateCoordinates(display, native, DefaultRootWindow(display),
                             (int)(position.x - offset_x),
                             (int)(position.y - offset_y), &root_x, &root_y,
                             &child))
    return 0;
  gtk_widget_grab_focus(GTK_WIDGET(view));
  XSetInputFocus(display, native, RevertToParent, CurrentTime);
  int ok = XTestFakeMotionEvent(display, -1, root_x, root_y, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, True, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, False, CurrentTime) &&
           XTestFakeKeyEvent(display, 38, True, CurrentTime) &&
           XTestFakeKeyEvent(display, 38, False, CurrentTime);
  XFlush(display);
  return ok != 0;
}

bool scoop_test_gtk_clipboard_nul(void *window) {
  static const char text[] = "left\0right";
  GBytes *bytes = g_bytes_new_static(text, sizeof(text) - 1);
  GdkContentProvider *provider =
      gdk_content_provider_new_for_bytes("text/plain;charset=utf-8", bytes);
  bool result = gdk_clipboard_set_content(
      gdk_display_get_clipboard(gtk_widget_get_display(GTK_WIDGET(window))),
      provider);
  g_object_unref(provider);
  g_bytes_unref(bytes);
  return result;
}
