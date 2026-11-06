#include "window.h"
#include <gdk/x11/gdkx.h>

// The window owns this container; a renderer may attach one content widget.
// Native popup menus use separate GtkPopover surfaces parented to this widget.
typedef struct {
  GtkWidget parent;
  SWWindow *owner;
} SWContent;
typedef GtkWidgetClass SWContentClass;
G_DEFINE_TYPE(SWContent, sw_content, GTK_TYPE_WIDGET)

static void content_measure(GtkWidget *widget, GtkOrientation orientation,
                            int for_size, int *minimum, int *natural,
                            int *minimum_baseline, int *natural_baseline) {
  SWWindow *window = ((SWContent *)widget)->owner;
  *minimum =
      (int)ceil(orientation == GTK_ORIENTATION_HORIZONTAL ? window->min_width
                                                          : window->min_height);
  *natural = (int)ceil(orientation == GTK_ORIENTATION_HORIZONTAL
                           ? window->requested_width
                           : window->requested_height);
  if (!window->resizable)
    *minimum = *natural;
  *natural = MAX(*minimum, *natural);
  *minimum_baseline = *natural_baseline = -1;
}
static void content_allocate(GtkWidget *widget, int width, int height,
                             int baseline) {
  SWWindow *window = ((SWContent *)widget)->owner;
  if (window->destroying)
    return;
  if (width > 0 && height > 0)
    sw_set_geometry(window, width, height, gtk_widget_get_scale_factor(widget));
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    if (GTK_IS_POPOVER(child))
      gtk_popover_present(GTK_POPOVER(child));
    else
      gtk_widget_allocate(child, width, height, baseline, NULL);
  }
}
static void content_snapshot(GtkWidget *widget, GtkSnapshot *snapshot) {
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child))
    gtk_widget_snapshot_child(widget, child, snapshot);
}
static void content_dispose(GObject *object) {
  GtkWidget *child;
  while ((child = gtk_widget_get_first_child(GTK_WIDGET(object))))
    gtk_widget_unparent(child);
  G_OBJECT_CLASS(sw_content_parent_class)->dispose(object);
}
static void sw_content_class_init(SWContentClass *klass) {
  klass->measure = content_measure;
  klass->size_allocate = content_allocate;
  klass->snapshot = content_snapshot;
  G_OBJECT_CLASS(klass)->dispose = content_dispose;
  gtk_widget_class_set_css_name(klass, "scoopcontent");
}
static void sw_content_init(SWContent *content) {
  gtk_widget_set_focusable(GTK_WIDGET(content), TRUE);
  gtk_widget_set_hexpand(GTK_WIDGET(content), TRUE);
  gtk_widget_set_vexpand(GTK_WIDGET(content), TRUE);
  gtk_widget_set_overflow(GTK_WIDGET(content), GTK_OVERFLOW_HIDDEN);
}
GdkSurface *sw_surface(SWWindow *window) {
  return gtk_native_get_surface(GTK_NATIVE(window->window));
}
void sw_snapshot_update(SWWindow *window) {
  if (window->destroying)
    return;
  GdkSurface *surface = sw_surface(window);
  GdkToplevelState state =
      surface ? gdk_toplevel_get_state(GDK_TOPLEVEL(surface)) : 0;
  window->snapshot.visible = gtk_widget_get_visible(window->window);
  window->snapshot.maximized = (state & GDK_TOPLEVEL_STATE_MAXIMIZED) != 0;
  window->snapshot.minimized = (state & GDK_TOPLEVEL_STATE_MINIMIZED) != 0;
  bool occluded =
      !gtk_widget_get_mapped(window->window) || window->snapshot.minimized;
  if (window->snapshot.occluded != occluded) {
    window->snapshot.occluded = occluded;
    sw_emit(window, 5);
  }
  sw_focus(window, gtk_window_is_active(GTK_WINDOW(window->window)));
  if (surface && GDK_IS_X11_SURFACE(surface)) {
    Display *display =
        gdk_x11_display_get_xdisplay(window->application->display);
    Window xid = gdk_x11_surface_get_xid(surface), child;
    int x, y;
    if (XTranslateCoordinates(display, xid, DefaultRootWindow(display), 0, 0,
                              &x, &y, &child)) {
      if (window->snapshot.x != x || window->snapshot.y != y) {
        window->snapshot.x = x;
        window->snapshot.y = y;
        sw_emit(window, 3);
      }
    }
  }
}
static void changed(GObject *object, GParamSpec *pspec, gpointer data) {
  sw_snapshot_update(data);
}
static void mapped(GtkWidget *widget, gpointer data) {
  SWWindow *window = data;
  sw_snapshot_update(window);
  if (gtk_widget_get_mapped(widget))
    gtk_widget_grab_focus(window->view);
}
static void layout(GdkSurface *surface, int width, int height, gpointer data) {
  sw_snapshot_update(data);
}
static gboolean close_requested(GtkWindow *native, gpointer data) {
  SWWindow *window = data;
  if (window->close_button)
    sw_emit(window, 1);
  return TRUE;
}
static void realized(GtkWidget *widget, gpointer data) {
  SWWindow *window = data;
  GdkSurface *surface = sw_surface(window);
  g_signal_connect(surface, "notify::state", G_CALLBACK(changed), window);
  g_signal_connect(surface, "layout", G_CALLBACK(layout), window);
}
void sw_theme_changed(AdwStyleManager *manager, GParamSpec *pspec,
                      gpointer data) {
  SWApplication *app = data;
  int dark = adw_style_manager_get_dark(manager);
  for (SWWindow *window = app->windows; window; window = window->next) {
    if (window->snapshot.dark != dark) {
      window->snapshot.dark = dark;
      sw_emit(window, 6);
    }
  }
}
SWResult scoop_window_create(SWApplication *app, int64_t identity,
                             const uint8_t *title, int64_t length, double width,
                             double height, int32_t flags) {
  if (app->error)
    return (SWResult){NULL, -3};
  if ((flags & 4) && app->kind == SW_X11 &&
      !gdk_display_is_composited(app->display))
    return (SWResult){NULL, -1};
  SWWindow *window = calloc(1, sizeof(*window));
  char *name = strndup((const char *)title, length);
  if (!window || !name) {
    free(window);
    free(name);
    return (SWResult){NULL, ENOMEM};
  }
  window->application = app;
  window->identity = identity;
  window->requested_width = width;
  window->requested_height = height;
  window->max_width = window->max_height = -1;
  window->increment_width = window->increment_height = 1;
  window->decorations = (flags & 1) != 0;
  window->resizable = (flags & 2) != 0;
  window->transparent = (flags & 4) != 0;
  window->close_button = window->minimize_button = window->maximize_button =
      true;
  window->cursor_visible = true;
  window->snapshot.scale = 1;
  window->snapshot.x = window->snapshot.y = NAN;
  window->snapshot.outer_width = window->snapshot.outer_height = NAN;
  window->snapshot.dark =
      adw_style_manager_get_dark(adw_style_manager_get_default());
  window->snapshot.occluded = true;
  window->document.revision = -1;
  window->window = adw_window_new();
  gtk_window_set_title(GTK_WINDOW(window->window), name);
  free(name);
  gtk_window_set_resizable(GTK_WINDOW(window->window), window->resizable);
  gtk_window_set_decorated(GTK_WINDOW(window->window), window->decorations);
  gtk_widget_add_css_class(window->window, "scoop-window");
  window->toolbar = adw_toolbar_view_new();
  window->header = adw_header_bar_new();
  adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(window->toolbar),
                               window->header);
  gtk_widget_set_visible(window->header, window->decorations);
  window->view = GTK_WIDGET(g_object_new(sw_content_get_type(), NULL));
  ((SWContent *)window->view)->owner = window;
  adw_toolbar_view_set_content(ADW_TOOLBAR_VIEW(window->toolbar), window->view);
  adw_window_set_content(ADW_WINDOW(window->window), window->toolbar);
  sw_transparency(window, window->transparent);
  sw_ime_install(window);
  sw_input_install(window);
  g_signal_connect(window->window, "close-request", G_CALLBACK(close_requested),
                   window);
  g_signal_connect(window->window, "realize", G_CALLBACK(realized), window);
  g_signal_connect(window->window, "notify::is-active", G_CALLBACK(changed),
                   window);
  g_signal_connect(window->window, "map", G_CALLBACK(mapped), window);
  g_signal_connect(window->window, "unmap", G_CALLBACK(mapped), window);
  gtk_widget_realize(window->window);
  window->next = app->windows;
  app->windows = window;
  return (SWResult){window, 0};
}
SWSnapshot scoop_window_snapshot(SWWindow *window) {
  sw_snapshot_update(window);
  return window->snapshot;
}
int32_t scoop_window_configured(SWWindow *window) { return window->configured; }
void scoop_window_destroy(SWWindow *window) {
  SWApplication *app = window->application;
  SWWindow **link = &app->windows;
  while (*link && *link != window)
    link = &(*link)->next;
  if (*link)
    *link = window->next;
  window->destroying = true;
  GdkSurface *surface = sw_surface(window);
  if (surface)
    g_signal_handlers_disconnect_by_data(surface, window);
  g_signal_handlers_disconnect_by_data(window->window, window);
  GListModel *controllers = gtk_widget_observe_controllers(window->view);
  for (guint i = 0; i < g_list_model_get_n_items(controllers); ++i) {
    GObject *controller = G_OBJECT(g_list_model_get_item(controllers, i));
    g_signal_handlers_disconnect_by_data(controller, window);
    g_object_unref(controller);
  }
  g_object_unref(controllers);
  g_signal_handlers_disconnect_by_data(window->ime, window);
  g_signal_handlers_disconnect_by_data(window->simple_ime, window);
  gtk_im_context_set_client_widget(window->ime, NULL);
  gtk_im_context_set_client_widget(window->simple_ime, NULL);
  g_clear_object(&window->ime);
  g_clear_object(&window->simple_ime);
  gtk_window_destroy(GTK_WINDOW(window->window));
  for (size_t i = 0; i < G_N_ELEMENTS(window->pressed); ++i)
    free((void *)window->pressed[i].text);
  free((void *)window->document.text);
  free(window->preedit);
  free(window);
}
int64_t scoop_window_title(SWWindow *window, const uint8_t *text,
                           int64_t length) {
  char *title = strndup((const char *)text, length);
  if (!title)
    return ENOMEM;
  gtk_window_set_title(GTK_WINDOW(window->window), title);
  free(title);
  return 0;
}
int64_t sw_request_size(SWWindow *window, double width, double height) {
  if (ceil(width * window->snapshot.scale) > INT_MAX ||
      ceil(height * window->snapshot.scale) > INT_MAX)
    return EOVERFLOW;
  window->requested_width = width;
  window->requested_height = height;
  gtk_widget_queue_resize(window->view);
  int minimum = 0, extra = 0;
  if (window->decorations)
    gtk_widget_measure(window->header, GTK_ORIENTATION_VERTICAL,
                       (int)ceil(width), &minimum, &extra, NULL, NULL);
  gtk_window_set_default_size(GTK_WINDOW(window->window), (int)ceil(width),
                              (int)ceil(height) + extra);
  return 0;
}
int64_t scoop_window_size(SWWindow *window, double width, double height) {
  return sw_request_size(window, width, height);
}
int64_t scoop_window_visible(SWWindow *window, int32_t visible) {
  window->requested_visible = visible != 0;
  gtk_widget_set_visible(window->window, visible != 0);
  sw_snapshot_update(window);
  sw_wake(window->application);
  return 0;
}
int64_t scoop_window_focus(SWWindow *window) {
  gtk_window_present(GTK_WINDOW(window->window));
  return 0;
}
void *scoop_window_gtk_window(SWWindow *window) { return window->window; }
void *scoop_window_gtk_view(SWWindow *window) { return window->view; }
