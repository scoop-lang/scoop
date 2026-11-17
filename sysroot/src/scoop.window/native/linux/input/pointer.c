#include "../window.h"

static void emit_pointer(SWWindow *window, SWPointerData data,
                         GdkEvent *native);

static void pointer_state(SWWindow *window, GtkEventController *controller,
                          double x, double y) {
  window->input.pointer_x = x;
  window->input.pointer_y = y;
  window->application->input.gdk_modifiers =
      gtk_event_controller_get_current_event_state(controller);
}
static void pointer_enter(GtkEventControllerMotion *controller, double x,
                          double y, gpointer data) {
  if (sw_emulated_pointer(GTK_EVENT_CONTROLLER(controller)))
    return;
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(controller), x, y);
  emit_pointer(window, (SWPointerData){.kind = SWPointerEntered}, NULL);
}
static void pointer_motion(GtkEventControllerMotion *controller, double x,
                           double y, gpointer data) {
  if (sw_emulated_pointer(GTK_EVENT_CONTROLLER(controller)))
    return;
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(controller), x, y);
  GdkEvent *event =
      gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(controller));
  emit_pointer(window, (SWPointerData){.kind = SWPointerMoved},
               event && (gdk_event_get_modifier_state(event) & GDK_BUTTON1_MASK)
                   ? event
                   : NULL);
}
static void pointer_leave(GtkEventControllerMotion *controller, gpointer data) {
  emit_pointer(data, (SWPointerData){.kind = SWPointerLeft}, NULL);
}
static int pointer_button(guint button) {
  return button == 1 ? 0 : button == 3 ? 1 : button == 2 ? 2 : (int)button;
}
static void pointer_pressed(GtkGestureClick *gesture, int count, double x,
                            double y, gpointer data) {
  if (sw_emulated_pointer(GTK_EVENT_CONTROLLER(gesture)))
    return;
  SWWindow *window = data;
  GdkEvent *event =
      gtk_event_controller_get_current_event(GTK_EVENT_CONTROLLER(gesture));
  if (sw_frame_pointer(window, event, x, y)) {
    gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_CLAIMED);
    return;
  }
  gtk_widget_grab_focus(window->view);
  pointer_state(window, GTK_EVENT_CONTROLLER(gesture), x, y);
  emit_pointer(window,
               (SWPointerData){.kind = SWPointerPressed,
                               .button = pointer_button(
                                   gtk_gesture_single_get_current_button(
                                       GTK_GESTURE_SINGLE(gesture))),
                               .click_count = count},
               event);
  gtk_gesture_set_state(GTK_GESTURE(gesture), GTK_EVENT_SEQUENCE_CLAIMED);
}
static void pointer_released(GtkGestureClick *gesture, int count, double x,
                             double y, gpointer data) {
  if (sw_emulated_pointer(GTK_EVENT_CONTROLLER(gesture)))
    return;
  SWWindow *window = data;
  pointer_state(window, GTK_EVENT_CONTROLLER(gesture), x, y);
  emit_pointer(window,
               (SWPointerData){.kind = SWPointerReleased,
                               .button = pointer_button(
                                   gtk_gesture_single_get_current_button(
                                       GTK_GESTURE_SINGLE(gesture))),
                               .click_count = count},
               NULL);
}
static void pointer_cancel(GtkGesture *gesture, GdkEventSequence *sequence,
                           gpointer data) {
  if (sequence)
    return;
  SWWindow *window = data;
  bool resetting = window->input.resetting_click;
  window->input.resetting_click = true;
  sw_cancel_pointer(window, SWPointerCaptureLost);
  window->input.resetting_click = resetting;
}
static gboolean scroll(GtkEventControllerScroll *controller, double dx,
                       double dy, gpointer data) {
  SWWindow *window = data;
  window->application->input.gdk_modifiers =
      gtk_event_controller_get_current_event_state(
          GTK_EVENT_CONTROLLER(controller));
  SWPointerData pointer = {.scale = window->snapshot.scale};
  sw_pointer_scroll(
      &pointer,
      gtk_event_controller_scroll_get_unit(controller) != GDK_SCROLL_UNIT_WHEEL,
      -dx, -dy, window->input.scroll_active ? SWScrollChanged : SWScrollNone,
      SWScrollNone);
  emit_pointer(window, pointer, NULL);
  return TRUE;
}
static void scroll_begin(GtkEventControllerScroll *controller, gpointer data) {
  SWWindow *window = data;
  window->input.scroll_active = true;
  emit_pointer(
      window,
      (SWPointerData){.kind = SWPointerPixelWheel, .phase = SWScrollBegan},
      NULL);
}
static void scroll_end(GtkEventControllerScroll *controller, gpointer data) {
  SWWindow *window = data;
  if (!window->input.scroll_active)
    return;
  window->input.scroll_active = false;
  emit_pointer(
      window,
      (SWPointerData){.kind = SWPointerPixelWheel, .phase = SWScrollEnded},
      NULL);
}

void sw_pointer_install(SWWindow *window) {
  GtkEventController *motion = gtk_event_controller_motion_new();
  g_signal_connect(motion, "enter", G_CALLBACK(pointer_enter), window);
  g_signal_connect(motion, "motion", G_CALLBACK(pointer_motion), window);
  g_signal_connect(motion, "leave", G_CALLBACK(pointer_leave), window);
  gtk_widget_add_controller(window->view, motion);
  GtkGesture *click = gtk_gesture_click_new();
  window->input.click_controller = GTK_EVENT_CONTROLLER(g_object_ref(click));
  gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(click), 0);
  g_signal_connect(click, "pressed", G_CALLBACK(pointer_pressed), window);
  g_signal_connect(click, "released", G_CALLBACK(pointer_released), window);
  g_signal_connect(click, "cancel", G_CALLBACK(pointer_cancel), window);
  gtk_widget_add_controller(window->view, GTK_EVENT_CONTROLLER(click));
  GtkEventController *wheel =
      gtk_event_controller_scroll_new(GTK_EVENT_CONTROLLER_SCROLL_BOTH_AXES);
  g_signal_connect(wheel, "scroll", G_CALLBACK(scroll), window);
  g_signal_connect(wheel, "scroll-begin", G_CALLBACK(scroll_begin), window);
  g_signal_connect(wheel, "scroll-end", G_CALLBACK(scroll_end), window);
  gtk_widget_add_controller(window->view, wheel);
}

void sw_cancel_pointer(SWWindow *window, int reason) {
  bool pressed = window->input.pressed_buttons != 0;
  window->input.pressed_buttons = 0;
  memset(window->input.pressed_clicks, 0, sizeof(window->input.pressed_clicks));
  if (pressed) {
    SWEvent *event = sw_event(window, SWPointer);
    if (event) {
      event->pointer.kind = SWPointerCancelled;
      event->pointer.cancel_reason = reason;
      sw_enqueue(window->application, event);
    }
  }
  if (window->input.scroll_active) {
    window->input.scroll_active = false;
    emit_pointer(window,
                 (SWPointerData){.kind = SWPointerPixelWheel,
                                 .phase = SWScrollCancelled},
                 NULL);
  }
  if (window->input.click_controller && !window->input.resetting_click) {
    window->input.resetting_click = true;
    gtk_event_controller_reset(window->input.click_controller);
    window->input.resetting_click = false;
  }
}

static void emit_pointer(SWWindow *window, SWPointerData data,
                         GdkEvent *native) {
  int kind = data.kind;
  int button = data.button;
  if (kind == SWPointerReleased && button >= 0 && button < 32 &&
      !(window->input.pressed_buttons & (1u << button)))
    return;
  SWEvent *event = sw_event(window, SWPointer);
  if (!event)
    return;
  data.x = window->input.pointer_x;
  data.y = window->input.pointer_y;
  data.scale = window->snapshot.scale;
  data.physical_x = data.x * data.scale;
  data.physical_y = data.y * data.scale;
  data.modifiers = sw_modifiers(window->application);
  if (kind == SWPointerEntered)
    window->input.pointer_inside = true;
  if (kind == SWPointerLeft)
    window->input.pointer_inside = false;
  if (kind == SWPointerPressed) {
    if (native)
      event->native_input = gdk_event_ref(native);
    data.click_count = data.click_count > 0 ? data.click_count : 1;
    if (button >= 0 && button < 32) {
      window->input.pressed_clicks[button] = data.click_count;
      window->input.pressed_buttons |= 1u << button;
    }
  }
  if (kind == SWPointerReleased) {
    data.click_count = data.click_count > 0 ? data.click_count : 1;
    if (button >= 0 && button < 32) {
      data.click_count = window->input.pressed_clicks[button];
      window->input.pressed_clicks[button] = 0;
      window->input.pressed_buttons &= ~(1u << button);
    }
  }
  event->pointer = data;
  sw_enqueue(window->application, event);
}

static void cursor(SWWindow *window) {
  const char *names[] = {"default",     "text",      "crosshair",
                         "pointer",     "grab",      "grabbing",
                         "not-allowed", "ew-resize", "ns-resize"};
  gtk_widget_set_cursor_from_name(
      window->view,
      window->input.cursor_visible ? names[window->input.cursor_icon] : "none");
}
int64_t scoop_window_cursor_icon(SWWindow *window, int32_t value) {
  window->input.cursor_icon = value;
  cursor(window);
  return 0;
}
int64_t scoop_window_cursor_visible(SWWindow *window, int32_t value) {
  window->input.cursor_visible = value != 0;
  cursor(window);
  return 0;
}
