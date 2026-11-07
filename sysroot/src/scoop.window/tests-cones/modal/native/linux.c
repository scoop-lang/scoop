#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#include <X11/Xatom.h>
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gdk/x11/gdkx.h>
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <stdint.h>

static int tool_window(GtkWindow *window) {
  GdkDisplay *display = gtk_widget_get_display(GTK_WIDGET(window));
  if (!GDK_IS_X11_DISPLAY(display))
    return 0;
  Display *xdisplay = gdk_x11_display_get_xdisplay(display);
  Atom actual;
  int format;
  unsigned long count, remaining;
  unsigned char *bytes = NULL;
  XGetWindowProperty(
      xdisplay,
      gdk_x11_surface_get_xid(gtk_native_get_surface(GTK_NATIVE(window))),
      XInternAtom(xdisplay, "_NET_WM_WINDOW_TYPE", False), 0, 8, False, XA_ATOM,
      &actual, &format, &count, &remaining, &bytes);
  int found = 0;
  if (format == 32 && actual == XA_ATOM) {
    Atom utility = XInternAtom(xdisplay, "_NET_WM_WINDOW_TYPE_UTILITY", False);
    for (unsigned long index = 0; index < count; ++index)
      if (((Atom *)bytes)[index] == utility)
        found = 1;
  }
  if (bytes)
    XFree(bytes);
  return found;
}
int32_t scoop_test_modal_query(GtkWindow *window, GtkWindow *parent,
                               int32_t field, int32_t value) {
  switch (field) {
  case 0:
    return gtk_window_get_transient_for(window) == parent &&
           gtk_window_get_group(window) != gtk_window_get_group(parent);
  case 1:
    return gtk_widget_get_sensitive(GTK_WIDGET(window));
  case 2:
    return tool_window(window);
  case 3:
    return 0;
  case 4:
    gtk_window_close(window);
    return 1;
  case 5:
    return 0;
  case 10:
    gtk_widget_set_sensitive(GTK_WIDGET(window), value != 0);
    return 1;
  case 20:
    return !GDK_IS_X11_DISPLAY(gtk_widget_get_display(GTK_WIDGET(window)));
  default:
    return -1;
  }
}
