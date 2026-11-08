#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gdk/x11/gdkx.h>
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <X11/Xatom.h>
#include <stdint.h>
#include <string.h>

int32_t scoop_desktop_test_application(int32_t field) {
  return field == 0 ? !g_strcmp0(g_get_prgname(), "dev.scoop.desktop.tests")
                    : -1;
}
int32_t scoop_desktop_test_window(void *pointer, int32_t field) {
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(pointer));
  if (field || !GDK_IS_X11_SURFACE(surface))
    return -1;
  Display *display =
      gdk_x11_display_get_xdisplay(gdk_surface_get_display(surface));
  Atom actual;
  int format;
  unsigned long count, remaining;
  unsigned char *data = NULL;
  int result = XGetWindowProperty(display, gdk_x11_surface_get_xid(surface),
                                  XInternAtom(display, "_NET_WM_ICON", False),
                                  0, 4096, False, XA_CARDINAL, &actual, &format,
                                  &count, &remaining, &data);
  int32_t present = result == Success && format == 32 && count > 0;
  if (data)
    XFree(data);
  return present;
}
int32_t scoop_desktop_test_dock(void) { return -1; }
int32_t scoop_desktop_test_dock_popup(void *window) { return -1; }
