#ifndef SCOOP_WINDOW_LINUX_INPUT_POSITION_H
#define SCOOP_WINDOW_LINUX_INPUT_POSITION_H

#include <gtk/gtk.h>
#include <math.h>
#include <stdbool.h>

static inline bool sw_input_position(GtkWidget *view, GdkEvent *event,
                                     double *x, double *y) {
  double surface_x, surface_y, dx, dy;
  if (!gdk_event_get_position(event, &surface_x, &surface_y))
    return false;
  GtkNative *native = gtk_widget_get_native(view);
  if (!native || gtk_native_get_surface(native) != gdk_event_get_surface(event))
    return false;
  gtk_native_get_surface_transform(native, &dx, &dy);
  // Match GTK's event dispatch: remove the native surface's shadow offset
  // before translating from the native widget to the content widget.
  graphene_point_t source = GRAPHENE_POINT_INIT(surface_x - dx, surface_y - dy);
  graphene_point_t target;
  if (!gtk_widget_compute_point(GTK_WIDGET(native), view, &source, &target))
    return false;
  *x = target.x;
  *y = target.y;
  return isfinite(*x) && isfinite(*y);
}

#endif
