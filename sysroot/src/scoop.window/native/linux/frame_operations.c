#include "window.h"

typedef struct {
  int64_t window;
  GdkEvent *event;
} SWFrameInput;
typedef struct {
  SWWindow *window;
  GdkEvent *event;
  int32_t kind, edge;
} SWFrameOperation;

SWResult scoop_window_input_copy(SWEvent *event) {
  if (!event->native_input)
    return (SWResult){NULL, 0};
  SWFrameInput *input = malloc(sizeof(*input));
  if (!input)
    return (SWResult){NULL, ENOMEM};
  *input =
      (SWFrameInput){event->data.window, gdk_event_ref(event->native_input)};
  return (SWResult){input, 0};
}
void scoop_window_input_release(SWFrameInput *input) {
  gdk_event_unref(input->event);
  free(input);
}

SWResult scoop_window_system_operation_prepare(SWWindow *window,
                                               SWFrameInput *input,
                                               int32_t kind, int32_t edge) {
  GdkEvent *event = input->event;
  if (input->window != window->identity ||
      gdk_event_get_surface(event) != sw_surface(window) ||
      gdk_event_get_event_type(event) != GDK_BUTTON_PRESS ||
      !gdk_event_get_device(event) || window->ignore_mouse ||
      (kind != 2 && gdk_button_event_get_button(event) != 1) ||
      (kind == 1 && (!window->resizable || window->snapshot.maximized)))
    return (SWResult){NULL, -4};
  SWFrameOperation *operation = malloc(sizeof(*operation));
  if (!operation)
    return (SWResult){NULL, ENOMEM};
  *operation = (SWFrameOperation){window, gdk_event_ref(event), kind, edge};
  window->pressed_buttons = 0;
  return (SWResult){operation, 0};
}

int64_t scoop_window_system_operation_run(SWFrameOperation *operation) {
  GdkToplevel *toplevel = GDK_TOPLEVEL(sw_surface(operation->window));
  GdkEvent *event = operation->event;
  if (operation->kind == 2) {
    // The WM may decline the request; Requested never promises that a menu
    // appeared.
    gdk_toplevel_show_window_menu(toplevel, event);
    return 0;
  }
  double x, y;
  gdk_event_get_position(event, &x, &y);
  if (operation->kind == 0)
    gdk_toplevel_begin_move(toplevel, gdk_event_get_device(event),
                            gdk_button_event_get_button(event), x, y,
                            gdk_event_get_time(event));
  else {
    const GdkSurfaceEdge edges[] = {
        GDK_SURFACE_EDGE_WEST,       GDK_SURFACE_EDGE_EAST,
        GDK_SURFACE_EDGE_NORTH,      GDK_SURFACE_EDGE_SOUTH,
        GDK_SURFACE_EDGE_NORTH_WEST, GDK_SURFACE_EDGE_NORTH_EAST,
        GDK_SURFACE_EDGE_SOUTH_WEST, GDK_SURFACE_EDGE_SOUTH_EAST};
    gdk_toplevel_begin_resize(
        toplevel, edges[operation->edge], gdk_event_get_device(event),
        gdk_button_event_get_button(event), x, y, gdk_event_get_time(event));
  }
  return 0;
}

void scoop_window_system_operation_release(SWFrameOperation *operation) {
  gdk_event_unref(operation->event);
  free(operation);
}
