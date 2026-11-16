#include "../window.h"
#include <math.h>

static void enqueue_contact(SWWindow *window, SWContactData data) {
  SWEvent *event = sw_event(window, SWContact);
  if (event) {
    event->contact = data;
    sw_enqueue(window->application, event);
  }
}

static void cancel_contact(SWWindow *window, SWContactEntry *entry,
                           int reason) {
  SWContactData data = entry->data;
  sw_contact_remove(&window->input.contacts, entry);
  data.phase = SWContactCancelled;
  data.cancel_reason = reason;
  enqueue_contact(window, data);
}

void sw_cancel_contacts(SWWindow *window, int reason) {
  while (window->input.contacts)
    cancel_contact(window, window->input.contacts, reason);
}

bool sw_emulated_pointer(GtkEventController *controller) {
  GdkEvent *event = gtk_event_controller_get_current_event(controller);
  if (!event)
    return false;
  GdkEventType type = gdk_event_get_event_type(event);
  return type == GDK_TOUCH_BEGIN || type == GDK_TOUCH_UPDATE ||
         type == GDK_TOUCH_END || type == GDK_TOUCH_CANCEL ||
         gdk_event_get_pointer_emulated(event);
}

static bool content_position(SWWindow *window, GdkEvent *event, double *x,
                             double *y) {
  double surface_x, surface_y, dx, dy;
  if (!gdk_event_get_position(event, &surface_x, &surface_y))
    return false;
  GtkNative *native = gtk_widget_get_native(window->view);
  if (!native || gtk_native_get_surface(native) != gdk_event_get_surface(event))
    return false;
  gtk_native_get_surface_transform(native, &dx, &dy);
  graphene_point_t source = GRAPHENE_POINT_INIT(surface_x + dx, surface_y + dy);
  graphene_point_t target;
  if (!gtk_widget_compute_point(GTK_WIDGET(native), window->view, &source,
                                &target))
    return false;
  *x = target.x;
  *y = target.y;
  return isfinite(*x) && isfinite(*y);
}

static gboolean contact_event(GtkEventControllerLegacy *controller,
                              GdkEvent *event, gpointer user_data) {
  (void)controller;
  SWWindow *window = user_data;
  GdkEventType type = gdk_event_get_event_type(event);
  if (type == GDK_GRAB_BROKEN) {
    sw_cancel_contacts(window, SWPointerCaptureLost);
    return FALSE;
  }
  bool touch = type == GDK_TOUCH_BEGIN || type == GDK_TOUCH_UPDATE ||
               type == GDK_TOUCH_END || type == GDK_TOUCH_CANCEL;
  GdkDeviceTool *tool = gdk_event_get_device_tool(event);
  if (!touch &&
      (!tool || (type != GDK_BUTTON_PRESS && type != GDK_BUTTON_RELEASE &&
                 type != GDK_MOTION_NOTIFY)))
    return FALSE;
  if (!touch && type != GDK_MOTION_NOTIFY &&
      gdk_button_event_get_button(event) != 1)
    return FALSE;
  if (window->input_blocked)
    return TRUE;
  int64_t id =
      (int64_t)(uintptr_t)(touch ? (void *)gdk_event_get_event_sequence(event)
                                 : (void *)tool);
  SWContactEntry *entry = sw_contact_find(window->input.contacts, id);
  if (type == GDK_TOUCH_CANCEL) {
    if (entry)
      cancel_contact(window, entry, SWPointerCaptureLost);
    return TRUE;
  }
  bool began = type == GDK_TOUCH_BEGIN || type == GDK_BUTTON_PRESS;
  bool ended = type == GDK_TOUCH_END || type == GDK_BUTTON_RELEASE;
  if (began && entry) {
    cancel_contact(window, entry, SWPointerCaptureLost);
    entry = NULL;
  }
  if (!began && !entry)
    return TRUE;
  double x, y;
  if (!content_position(window, event, &x, &y)) {
    if (entry)
      cancel_contact(window, entry, SWPointerCaptureLost);
    return TRUE;
  }
  if (began && sw_frame_pointer(window, event, x, y))
    return TRUE;
  if (began)
    gtk_widget_grab_focus(window->view);
  window->application->input.gdk_modifiers =
      gdk_event_get_modifier_state(event);
  double scale = window->snapshot.scale;
  GdkDevice *device = gdk_event_get_device(event);
  SWContactData data = {.id = id,
                        .device = (int64_t)(uintptr_t)device,
                        .modifiers = (int64_t)sw_modifiers(window->application),
                        .kind = touch ? SWContactTouch : SWContactPen,
                        .phase = began   ? SWContactBegan
                                 : ended ? SWContactEnded
                                         : SWContactMoved,
                        .has_device = device != NULL,
                        .x = x,
                        .y = y,
                        .physical_x = x * scale,
                        .physical_y = y * scale,
                        .scale = scale};
  double pressure;
  if (gdk_event_get_axis(event, GDK_AXIS_PRESSURE, &pressure) &&
      isfinite(pressure) && pressure >= 0 && pressure <= 1) {
    data.has_pressure = true;
    data.pressure = pressure;
  }
  if (began) {
    entry = sw_contact_add(&window->input.contacts, data);
    if (!entry) {
      sw_fail(window->application, ENOMEM);
      return TRUE;
    }
  } else
    entry->data = data;
  if (ended)
    sw_contact_remove(&window->input.contacts, entry);
  enqueue_contact(window, data);
  return TRUE;
}

void sw_contact_install(SWWindow *window) {
  // UI recognition needs each raw sequence; GTK gesture recognizers would
  // choose winners before Surface can route and claim the contacts.
  GtkEventController *controller = gtk_event_controller_legacy_new();
  gtk_event_controller_set_propagation_phase(controller, GTK_PHASE_CAPTURE);
  g_signal_connect(controller, "event", G_CALLBACK(contact_event), window);
  gtk_widget_add_controller(window->view, controller);
}
