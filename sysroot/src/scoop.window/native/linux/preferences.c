#include "../preferences.h"
#include "window.h"

SWInteractionPreferences scoop_window_interaction_preferences(void *pointer) {
  SWInteractionPreferences value = sw_unknown_preferences();
  GtkSettings *settings = gtk_settings_get_default();
  gboolean overlay, animations, blink;
  gint blink_time, double_click_time;
  g_object_get(settings, "gtk-overlay-scrolling", &overlay,
               "gtk-enable-animations", &animations, "gtk-cursor-blink", &blink,
               "gtk-cursor-blink-time", &blink_time, "gtk-double-click-time",
               &double_click_time, NULL);
  value.auto_hide_scrollbars = overlay;
  value.reduce_motion = !animations;
  value.high_contrast =
      adw_style_manager_get_high_contrast(adw_style_manager_get_default());
  value.caret_blink_seconds = blink ? blink_time / 2000.0 : 0;
  value.double_click_seconds = double_click_time / 1000.0;
  GParamSpec *aspect_property = g_object_class_find_property(
      G_OBJECT_GET_CLASS(settings), "gtk-cursor-aspect-ratio");
  if (aspect_property) {
    GValue aspect = G_VALUE_INIT;
    g_value_init(&aspect, G_PARAM_SPEC_VALUE_TYPE(aspect_property));
    g_object_get_property(G_OBJECT(settings), "gtk-cursor-aspect-ratio",
                          &aspect);
    if (G_VALUE_HOLDS_DOUBLE(&aspect))
      value.caret_aspect_ratio = g_value_get_double(&aspect);
    else if (G_VALUE_HOLDS_FLOAT(&aspect))
      value.caret_aspect_ratio = g_value_get_float(&aspect);
    g_value_unset(&aspect);
  }
  return value;
}

static void preferences_changed(GObject *object, GParamSpec *spec,
                                gpointer data) {
  static const char *names[] = {"gtk-overlay-scrolling",
                                "gtk-enable-animations",
                                "gtk-cursor-blink",
                                "gtk-cursor-blink-time",
                                "gtk-cursor-aspect-ratio",
                                "gtk-double-click-time",
                                "high-contrast"};
  for (size_t index = 0; index < G_N_ELEMENTS(names); ++index) {
    if (strcmp(spec->name, names[index]))
      continue;
    SWApplication *app = data;
    SWEvent *event = calloc(1, sizeof(*event));
    if (!event) {
      sw_fail(app, ENOMEM);
      return;
    }
    event->header.kind = 102;
    sw_enqueue(app, event);
    return;
  }
}

void sw_preferences_start(SWApplication *app) {
  app->preferences_changed =
      g_signal_connect(gtk_settings_get_default(), "notify",
                       G_CALLBACK(preferences_changed), app);
  app->contrast_changed =
      g_signal_connect(adw_style_manager_get_default(), "notify::high-contrast",
                       G_CALLBACK(preferences_changed), app);
}

void sw_preferences_stop(SWApplication *app) {
  if (app->preferences_changed)
    g_signal_handler_disconnect(gtk_settings_get_default(),
                                app->preferences_changed);
  if (app->contrast_changed)
    g_signal_handler_disconnect(adw_style_manager_get_default(),
                                app->contrast_changed);
}
