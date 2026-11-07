#include "window.h"
#include <gdk/x11/gdkx.h>

static void x11_state(SWWindow *window, bool enabled, const char *name) {
  Display *display = gdk_x11_display_get_xdisplay(window->application->display);
  XEvent event = {0};
  event.xclient.type = ClientMessage;
  event.xclient.window = gdk_x11_surface_get_xid(sw_surface(window));
  event.xclient.message_type = XInternAtom(display, "_NET_WM_STATE", False);
  event.xclient.format = 32;
  event.xclient.data.l[0] = enabled ? 1 : 0;
  event.xclient.data.l[1] = XInternAtom(display, name, False);
  event.xclient.data.l[3] = 1;
  XSendEvent(display, DefaultRootWindow(display), False,
             SubstructureRedirectMask | SubstructureNotifyMask, &event);
  XFlush(display);
}
int64_t scoop_window_constraints(SWWindow *window, double min_width,
                                 double min_height, double max_width,
                                 double max_height) {
  if (min_width > INT_MAX || min_height > INT_MAX || max_width > INT_MAX ||
      max_height > INT_MAX)
    return EOVERFLOW;
  if (max_width >= 0 || max_height >= 0)
    return -1;
  window->min_width = min_width;
  window->min_height = min_height;
  gtk_widget_queue_resize(window->view);
  return 0;
}
int64_t scoop_window_resize_increments(SWWindow *window, double width,
                                       double height) {
  return width == 1 && height == 1 ? 0 : -1;
}
int64_t scoop_window_resizable(SWWindow *window, int32_t value) {
  window->resizable = value != 0;
  gtk_window_set_resizable(GTK_WINDOW(window->window), window->resizable);
  gtk_widget_queue_resize(window->view);
  return 0;
}
void sw_update_buttons(SWWindow *window) {
  if (window->close_button && window->minimize_button &&
      window->maximize_button) {
    adw_header_bar_set_decoration_layout(ADW_HEADER_BAR(window->header), NULL);
    return;
  }
  char *layout = NULL;
  g_object_get(gtk_widget_get_settings(window->window), "gtk-decoration-layout",
               &layout, NULL);
  gchar **sides =
      g_strsplit(layout ? layout : ":minimize,maximize,close", ":", 2);
  GString *filtered = g_string_new(NULL);
  for (unsigned side = 0; side < 2; ++side) {
    if (side)
      g_string_append_c(filtered, ':');
    gchar **buttons = g_strsplit(sides[side] ? sides[side] : "", ",", -1);
    bool first = true;
    for (char **button = buttons; *button; ++button) {
      if ((!strcmp(*button, "close") && !window->close_button) ||
          (!strcmp(*button, "minimize") && !window->minimize_button) ||
          (!strcmp(*button, "maximize") && !window->maximize_button) ||
          !**button)
        continue;
      if (!first)
        g_string_append_c(filtered, ',');
      g_string_append(filtered, *button);
      first = false;
    }
    g_strfreev(buttons);
    if (!sides[side])
      break;
  }
  adw_header_bar_set_decoration_layout(ADW_HEADER_BAR(window->header),
                                       filtered->str);
  g_string_free(filtered, TRUE);
  g_strfreev(sides);
  g_free(layout);
}
int64_t scoop_window_buttons(SWWindow *window, int32_t close, int32_t minimize,
                             int32_t maximize) {
  window->close_button = close != 0;
  window->minimize_button = minimize != 0;
  window->maximize_button = maximize != 0;
  gtk_window_set_deletable(GTK_WINDOW(window->window), window->close_button);
  sw_update_buttons(window);
  return 0;
}
int64_t scoop_window_level(SWWindow *window, int32_t value) {
  if (window->application->kind != SW_X11)
    return value ? -1 : 0;
  x11_state(window, value > 0, "_NET_WM_STATE_ABOVE");
  x11_state(window, value < 0, "_NET_WM_STATE_BELOW");
  return 0;
}
int64_t scoop_window_minimized(SWWindow *window, int32_t value) {
  if (value)
    gtk_window_minimize(GTK_WINDOW(window->window));
  else
    gtk_window_unminimize(GTK_WINDOW(window->window));
  return 0;
}
int64_t scoop_window_maximized(SWWindow *window, int32_t value) {
  if (value)
    gtk_window_maximize(GTK_WINDOW(window->window));
  else
    gtk_window_unmaximize(GTK_WINDOW(window->window));
  return 0;
}
int64_t scoop_window_theme(SWWindow *window, int32_t value) {
  return value < 0 ? 0 : -1;
}
int64_t sw_transparency(SWWindow *window, bool transparent) {
  static bool installed;
  if (!installed) {
    GtkCssProvider *provider = gtk_css_provider_new();
    gtk_css_provider_load_from_string(
        provider,
        ".scoop-window.scoop-transparent, "
        ".scoop-window.scoop-transparent contents, "
        ".scoop-window.scoop-transparent toolbarview, "
        ".scoop-window.scoop-transparent scoopcontent "
        "{ background: transparent; }"
        ".scoop-window.scoop-client, .scoop-window.scoop-client contents "
        "{ border-radius: 0; border: 0 solid transparent; padding: 0; "
        "box-shadow: none; }"
        ".scoop-window.scoop-no-shadow { box-shadow: none; }");
    gtk_style_context_add_provider_for_display(
        window->application->display, GTK_STYLE_PROVIDER(provider),
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
    g_object_unref(provider);
    installed = true;
  }
  if (transparent)
    gtk_widget_add_css_class(window->window, "scoop-transparent");
  else
    gtk_widget_remove_css_class(window->window, "scoop-transparent");
  return 0;
}
int64_t scoop_window_transparent(SWWindow *window, int32_t value) {
  if (value && window->application->kind == SW_X11 &&
      !gdk_display_is_composited(window->application->display))
    return -1;
  window->transparent = value != 0;
  return sw_transparency(window, window->transparent);
}
int64_t scoop_window_blur(SWWindow *window, int32_t value) {
  return value ? -1 : 0;
}
int64_t scoop_window_content_protection(SWWindow *window, int32_t value) {
  return value ? -1 : 0;
}
int64_t scoop_window_attention(SWWindow *window, int32_t value) {
  if (window->application->kind != SW_X11)
    return value < 0 ? 0 : -1;
  gdk_x11_surface_set_urgency_hint(sw_surface(window), value >= 0);
  return 0;
}
static void cursor(SWWindow *window) {
  const char *names[] = {"default",     "text",      "crosshair",
                         "pointer",     "grab",      "grabbing",
                         "not-allowed", "ew-resize", "ns-resize"};
  gtk_widget_set_cursor_from_name(window->view, window->cursor_visible
                                                    ? names[window->cursor_icon]
                                                    : "none");
}
int64_t scoop_window_cursor_icon(SWWindow *window, int32_t value) {
  window->cursor_icon = value;
  cursor(window);
  return 0;
}
int64_t scoop_window_cursor_visible(SWWindow *window, int32_t value) {
  window->cursor_visible = value != 0;
  cursor(window);
  return 0;
}
