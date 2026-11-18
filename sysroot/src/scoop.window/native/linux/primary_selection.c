#include "window.h"

typedef struct {
  int64_t generation, error;
} SWPrimaryClaim;

static GdkClipboard *primary(SWApplication *app) {
  if (!app->display || gdk_display_is_closed(app->display))
    return NULL;
  GdkClipboard *clipboard = gdk_display_get_primary_clipboard(app->display);
  // GTK uses the base class as a local-only fallback when the protocol is
  // absent.
  return G_OBJECT_TYPE(clipboard) == GDK_TYPE_CLIPBOARD ? NULL : clipboard;
}

static void primary_changed(GdkClipboard *clipboard, gpointer data) {
  SWApplication *app = data;
  if (app->primary_provider &&
      gdk_clipboard_get_content(clipboard) != app->primary_provider)
    g_clear_object(&app->primary_provider);
}

void sw_primary_cleanup(SWApplication *app) {
  if (app->primary_changed)
    g_signal_handler_disconnect(app->primary_clipboard, app->primary_changed);
  app->primary_changed = 0;
  if (app->primary_provider &&
      !gdk_display_is_closed(
          gdk_clipboard_get_display(app->primary_clipboard)) &&
      gdk_clipboard_get_content(app->primary_clipboard) ==
          app->primary_provider)
    gdk_clipboard_set_content(app->primary_clipboard, NULL);
  g_clear_object(&app->primary_provider);
  g_clear_object(&app->primary_clipboard);
}

int32_t scoop_window_primary_flags(SWApplication *app) {
  if (!primary(app))
    return 0;
  gboolean enabled = TRUE;
  GtkSettings *settings = gtk_settings_get_default();
  if (settings)
    g_object_get(settings, "gtk-enable-primary-paste", &enabled, NULL);
  return enabled ? 3 : 1;
}

SWClipboardText scoop_window_primary_read(SWApplication *app) {
  if (app->error)
    return (SWClipboardText){.error = -3};
  GdkClipboard *clipboard = primary(app);
  if (!clipboard)
    return (SWClipboardText){0};
  return sw_read_clipboard(app, clipboard, (const uint8_t *)"text/plain", 10);
}

SWPrimaryClaim scoop_window_primary_claim(SWApplication *app,
                                          const uint8_t *text, int64_t length) {
  if (app->error)
    return (SWPrimaryClaim){0, -3};
  GdkClipboard *clipboard = primary(app);
  if (!clipboard)
    return (SWPrimaryClaim){0};
  if (app->primary_generation == INT64_MAX)
    return (SWPrimaryClaim){0, EOVERFLOW};
  char *value = strndup((const char *)text, (size_t)length);
  if (!value)
    return (SWPrimaryClaim){0, ENOMEM};
  GdkContentProvider *provider =
      gdk_content_provider_new_typed(G_TYPE_STRING, value);
  free(value);
  if (app->primary_clipboard != clipboard) {
    sw_primary_cleanup(app);
    app->primary_clipboard = g_object_ref(clipboard);
    app->primary_changed = g_signal_connect(clipboard, "changed",
                                            G_CALLBACK(primary_changed), app);
  }
  gboolean claimed = gdk_clipboard_set_content(clipboard, provider);
  if (claimed) {
    g_set_object(&app->primary_provider, provider);
    ++app->primary_generation;
  }
  g_object_unref(provider);
  return claimed ? (SWPrimaryClaim){app->primary_generation, 0}
                 : (SWPrimaryClaim){0, EIO};
}

int32_t scoop_window_primary_current(SWApplication *app, int64_t generation) {
  return app->primary_provider && generation == app->primary_generation &&
         !gdk_display_is_closed(
             gdk_clipboard_get_display(app->primary_clipboard)) &&
         gdk_clipboard_get_content(app->primary_clipboard) ==
             app->primary_provider;
}

int64_t scoop_window_primary_release(SWApplication *app, int64_t generation) {
  if (generation != app->primary_generation || !app->primary_provider)
    return 0;
  gboolean released = TRUE;
  if (scoop_window_primary_current(app, generation))
    released = gdk_clipboard_set_content(app->primary_clipboard, NULL);
  g_clear_object(&app->primary_provider);
  return released ? 0 : EIO;
}
