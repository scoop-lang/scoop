#include "window.h"

int32_t scoop_window_frame_capabilities(void) {
  int32_t bits = SW_FRAME_CLIENT | SW_FRAME_EXTENDED | SW_FRAME_SYSTEM_RESIZE |
                 SW_FRAME_MOVE | SW_FRAME_RESIZE | SW_FRAME_MENU |
                 SW_FRAME_SHADOW | SW_FRAME_RESIZE_HIT;
  if (gdk_display_supports_input_shapes(sw_application->display))
    bits |= SW_FRAME_INPUT | SW_FRAME_IGNORE;
  return bits;
}

static bool content_offset(SWWindow *window, double *x, double *y) {
  graphene_point_t origin = GRAPHENE_POINT_INIT(0, 0), point;
  if (!gtk_widget_compute_point(window->view, window->window, &origin, &point))
    return false;
  double native_x, native_y;
  gtk_native_get_surface_transform(GTK_NATIVE(window->window), &native_x,
                                   &native_y);
  *x = point.x + native_x;
  *y = point.y + native_y;
  return true;
}

static void apply_input(SWWindow *window) {
  GdkSurface *surface = sw_surface(window);
  if (!surface)
    return;
  if (window->ignore_mouse) {
    cairo_region_t *empty = cairo_region_create();
    gdk_surface_set_input_region(surface, empty);
    cairo_region_destroy(empty);
  } else if (window->input_region) {
    gdk_surface_set_input_region(surface, window->input_region);
  }
}

static void restore_input(SWWindow *window) {
  GdkSurface *surface = sw_surface(window);
  if (!surface)
    return;
  gdk_surface_set_input_region(surface, NULL);
  // GtkWindow restores its shadow/resize input shape during the next
  // allocation, before after_layout reapplies any remaining override.
  gtk_widget_queue_allocate(window->window);
  apply_input(window);
}

void sw_frame_invalidate(SWWindow *window) {
  window->frame_revision += 1;
  free(window->hit_rects);
  window->hit_rects = NULL;
  window->hit_count = 0;
  if (window->input_region) {
    cairo_region_destroy(window->input_region);
    window->input_region = NULL;
    restore_input(window);
  }
  sw_emit(window, 17);
}

uint64_t scoop_window_frame_revision(SWWindow *window) {
  return window->frame_revision;
}

static void sync_geometry(SWWindow *window) {
  double x = 0, y = 0;
  if (window->configured && content_offset(window, &x, &y)) {
    SWSnapshot snapshot = window->snapshot;
    if (snapshot.width != window->frame_width ||
        snapshot.height != window->frame_height ||
        snapshot.scale != window->frame_scale || x != window->frame_x ||
        y != window->frame_y) {
      window->frame_width = snapshot.width;
      window->frame_height = snapshot.height;
      window->frame_scale = snapshot.scale;
      window->frame_x = x;
      window->frame_y = y;
      sw_frame_invalidate(window);
    }
  }
}

static gboolean after_layout(gpointer data) {
  SWWindow *window = data;
  window->frame_idle = 0;
  if (window->destroying)
    return G_SOURCE_REMOVE;
  sync_geometry(window);
  // GtkWindow writes input shape during allocation; apply the still-valid
  // override afterwards.
  apply_input(window);
  return G_SOURCE_REMOVE;
}

void sw_frame_layout(SWWindow *window) {
  if (!window->frame_idle && !window->destroying)
    window->frame_idle =
        g_idle_add_full(G_PRIORITY_HIGH_IDLE, after_layout, window, NULL);
}

int64_t scoop_window_frame(SWWindow *window, int32_t mode) {
  if (window->frame_mode == mode)
    return 0;
  window->frame_mode = mode;
  gtk_window_set_decorated(GTK_WINDOW(window->window), mode != 2);
  gtk_widget_set_visible(window->header, mode != 2);
  adw_toolbar_view_set_extend_content_to_top_edge(
      ADW_TOOLBAR_VIEW(window->toolbar), mode == 1);
  if (mode == 2)
    gtk_widget_add_css_class(window->window, "scoop-client");
  else
    gtk_widget_remove_css_class(window->window, "scoop-client");
  sw_frame_invalidate(window);
  return sw_request_size(window, window->requested_width,
                         window->requested_height);
}

SWFrameSnapshot scoop_window_frame_snapshot(SWWindow *window) {
  SWFrameRect layout = sw_frame_unknown();
  if (window->configured) {
    layout =
        (SWFrameRect){0, 0, window->snapshot.width, window->snapshot.height};
    if (window->frame_mode == 1) {
      graphene_point_t origin = GRAPHENE_POINT_INIT(0, 0), point;
      if (gtk_widget_compute_point(window->toolbar, window->view, &origin,
                                   &point)) {
        layout.y = fmax(0, point.y + adw_toolbar_view_get_top_bar_height(
                                         ADW_TOOLBAR_VIEW(window->toolbar)));
        layout.height = fmax(0, layout.height - layout.y);
      } else
        layout = sw_frame_unknown();
    }
  }
  return (SWFrameSnapshot){window->frame_mode, window->configured, layout, NAN};
}

static GtkWidget *find_button(GtkWidget *widget, int role) {
  const char *action =
      GTK_IS_ACTIONABLE(widget)
          ? gtk_actionable_get_action_name(GTK_ACTIONABLE(widget))
          : NULL;
  const char *expected = role == 0   ? "window.close"
                         : role == 1 ? "window.minimize"
                                     : "window.toggle-maximized";
  if (GTK_IS_BUTTON(widget) && action && !strcmp(action, expected))
    return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *button = find_button(child, role);
    if (button)
      return button;
  }
  return NULL;
}

SWFrameRect scoop_window_frame_button(SWWindow *window, int32_t role) {
  if (!window->configured || window->frame_mode == 2)
    return sw_frame_unknown();
  GtkWidget *button = find_button(window->header, role);
  graphene_rect_t rect;
  if (!button || !gtk_widget_is_visible(button) ||
      !gtk_widget_compute_bounds(button, window->view, &rect) ||
      rect.size.width <= 0 || rect.size.height <= 0)
    return sw_frame_unknown();
  return (SWFrameRect){rect.origin.x, rect.origin.y, rect.size.width,
                       rect.size.height};
}

int64_t scoop_window_shadow(SWWindow *window, int32_t enabled) {
  if (enabled)
    gtk_widget_remove_css_class(window->window, "scoop-no-shadow");
  else
    gtk_widget_add_css_class(window->window, "scoop-no-shadow");
  sw_frame_layout(window);
  return 0;
}

int64_t scoop_window_region(SWWindow *window, int32_t kind, int32_t present,
                            const SWFrameRect *rects, int64_t count) {
  if (kind != 1 ||
      !gdk_display_supports_input_shapes(window->application->display))
    return present ? -1 : 0;
  GdkSurface *surface = sw_surface(window);
  if (!surface || !window->configured)
    return -2;
  sync_geometry(window);
  cairo_region_t *region = NULL;
  if (present) {
    double dx, dy;
    if (!content_offset(window, &dx, &dy))
      return -2;
    region = cairo_region_create();
    for (int64_t i = 0; i < count; ++i) {
      if (rects[i].width == 0 || rects[i].height == 0)
        continue;
      // GDK regions use surface logical units, not framebuffer pixels.
      double left = floor(rects[i].x + dx), top = floor(rects[i].y + dy);
      double right = ceil(rects[i].x + rects[i].width + dx),
             bottom = ceil(rects[i].y + rects[i].height + dy);
      if (!sw_frame_rect_valid(rects[i]) || left < INT_MIN || top < INT_MIN ||
          right > INT_MAX || bottom > INT_MAX || right - left > INT_MAX ||
          bottom - top > INT_MAX) {
        cairo_region_destroy(region);
        return EOVERFLOW;
      }
      cairo_rectangle_int_t rect = {(int)left, (int)top, (int)(right - left),
                                    (int)(bottom - top)};
      cairo_status_t status = cairo_region_union_rectangle(region, &rect);
      if (status != CAIRO_STATUS_SUCCESS) {
        cairo_region_destroy(region);
        return ENOMEM;
      }
    }
    if (cairo_region_status(region) != CAIRO_STATUS_SUCCESS) {
      cairo_region_destroy(region);
      return ENOMEM;
    }
  }
  if (window->input_region)
    cairo_region_destroy(window->input_region);
  window->input_region = region;
  if (present)
    apply_input(window);
  else
    restore_input(window);
  return 0;
}

int64_t scoop_window_ignore_mouse(SWWindow *window, int32_t enabled) {
  if (!gdk_display_supports_input_shapes(window->application->display))
    return enabled ? -1 : 0;
  window->ignore_mouse = enabled != 0;
  if (enabled) {
    sw_cancel_pointer(window, 1);
    apply_input(window);
  } else
    restore_input(window);
  return 0;
}

int64_t scoop_window_hit_regions(SWWindow *window, const SWHitRect *rects,
                                 int64_t count) {
  sync_geometry(window);
  if (count < 0 || (uint64_t)count > SIZE_MAX / sizeof(*rects))
    return EOVERFLOW;
  for (int64_t i = 0; i < count; ++i) {
    if (!sw_frame_rect_valid(rects[i].rect))
      return EINVAL;
    if (rects[i].role < 0 || rects[i].role > 9)
      return -1;
  }
  SWHitRect *copy = count ? malloc((size_t)count * sizeof(*copy)) : NULL;
  if (count && !copy)
    return ENOMEM;
  if (count)
    memcpy(copy, rects, (size_t)count * sizeof(*copy));
  free(window->hit_rects);
  window->hit_rects = copy;
  window->hit_count = (size_t)count;
  return 0;
}

bool sw_frame_pointer(SWWindow *window, GdkEvent *event, double x, double y) {
  if (!event || gdk_event_get_event_type(event) != GDK_BUTTON_PRESS ||
      gdk_button_event_get_button(event) != 1)
    return false;
  int role = sw_frame_hit(window->hit_rects, window->hit_count, x, y);
  if (role == 0 ||
      (role >= 2 && (!window->resizable || window->snapshot.maximized)))
    return false;
  sw_cancel_pointer(window, 2);
  double native_x, native_y;
  gdk_event_get_position(event, &native_x, &native_y);
  if (role == 1)
    gdk_toplevel_begin_move(GDK_TOPLEVEL(sw_surface(window)),
                            gdk_event_get_device(event), 1, native_x, native_y,
                            gdk_event_get_time(event));
  else {
    const GdkSurfaceEdge edges[] = {
        GDK_SURFACE_EDGE_WEST,       GDK_SURFACE_EDGE_EAST,
        GDK_SURFACE_EDGE_NORTH,      GDK_SURFACE_EDGE_SOUTH,
        GDK_SURFACE_EDGE_NORTH_WEST, GDK_SURFACE_EDGE_NORTH_EAST,
        GDK_SURFACE_EDGE_SOUTH_WEST, GDK_SURFACE_EDGE_SOUTH_EAST};
    gdk_toplevel_begin_resize(GDK_TOPLEVEL(sw_surface(window)), edges[role - 2],
                              gdk_event_get_device(event), 1, native_x,
                              native_y, gdk_event_get_time(event));
  }
  return true;
}

void scoop_window_frame_retire(SWWindow *window) {
  free(window->hit_rects);
  window->hit_rects = NULL;
  window->hit_count = 0;
}
