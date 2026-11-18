#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include "../../native/linux/input/position.h"
#include "../../native/linux/window.h"
#include <X11/extensions/XTest.h>
#include <gdk/x11/gdkx.h>
#include <stdbool.h>
#include <stdint.h>

typedef struct {
  bool known;
  double x, y;
} ContactPositionProbe;

static gboolean record_contact_position(GtkEventControllerLegacy *controller,
                                        GdkEvent *event, gpointer data) {
  GdkEventType type = gdk_event_get_event_type(event);
  if (type == GDK_BUTTON_PRESS || type == GDK_BUTTON_RELEASE) {
    ContactPositionProbe position = {0};
    position.known = sw_input_position(
        gtk_event_controller_get_widget(GTK_EVENT_CONTROLLER(controller)),
        event, &position.x, &position.y);
    g_array_append_val((GArray *)data, position);
  }
  return FALSE;
}

bool scoop_test_gtk_contact_position(void *view, double x, double y) {
  GArray *positions =
      g_object_get_data(G_OBJECT(view), "scoop-test-contact-position");
  if (!positions || !positions->len)
    return false;
  ContactPositionProbe position =
      g_array_index(positions, ContactPositionProbe, 0);
  g_array_remove_index(positions, 0);
  return position.known && fabs(position.x - x) < 0.01 &&
         fabs(position.y - y) < 0.01;
}

bool scoop_test_gtk_scale_notification(SWWindow *window) {
  if (!window->configured)
    return false;
  SWSnapshot before = window->snapshot;
  // Recreate a previous backing configuration without changing desktop
  // settings.
  sw_set_geometry(window, before.width, before.height, before.scale + 1);
  g_object_notify(G_OBJECT(window->view), "scale-factor");
  SWSnapshot after = window->snapshot;
  return after.width == before.width && after.height == before.height &&
         after.scale == before.scale &&
         after.pixel_width == before.pixel_width &&
         after.pixel_height == before.pixel_height;
}

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
  double scale =
      gdk_surface_get_scale(gtk_native_get_surface(GTK_NATIVE(window)));
  int root_x, root_y;
  Window child;
  if (!XTranslateCoordinates(display, native, DefaultRootWindow(display),
                             (int)round((position.x + offset_x) * scale),
                             (int)round((position.y + offset_y) * scale),
                             &root_x, &root_y, &child))
    return 0;
  GArray *probe =
      g_object_get_data(G_OBJECT(view), "scoop-test-contact-position");
  if (!probe) {
    probe = g_array_new(FALSE, FALSE, sizeof(ContactPositionProbe));
    g_object_set_data_full(G_OBJECT(view), "scoop-test-contact-position", probe,
                           (GDestroyNotify)g_array_unref);
    GtkEventController *controller = gtk_event_controller_legacy_new();
    gtk_event_controller_set_propagation_phase(controller, GTK_PHASE_CAPTURE);
    g_signal_connect(controller, "event", G_CALLBACK(record_contact_position),
                     probe);
    gtk_widget_add_controller(GTK_WIDGET(view), controller);
  }
  g_array_set_size(probe, 0);
  gtk_widget_grab_focus(GTK_WIDGET(view));
  XSetInputFocus(display, native, RevertToParent, CurrentTime);
  int ok = XTestFakeMotionEvent(display, -1, root_x, root_y, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, True, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, False, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, True, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, False, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, True, CurrentTime) &&
           XTestFakeButtonEvent(display, 1, False, CurrentTime) &&
           XTestFakeKeyEvent(display, 38, True, CurrentTime) &&
           XTestFakeKeyEvent(display, 38, False, CurrentTime) &&
           XTestFakeButtonEvent(display, 5, True, CurrentTime) &&
           XTestFakeButtonEvent(display, 5, False, CurrentTime) &&
           XTestFakeButtonEvent(display, 7, True, CurrentTime) &&
           XTestFakeButtonEvent(display, 7, False, CurrentTime);
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
