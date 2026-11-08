#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gdk/x11/gdkx.h>
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include "../desktop.h"
#include <errno.h>

typedef struct {
  SDCallback callback;
  void *context;
} SDApplication;
typedef struct {
  GtkWindow *window;
  SDImage *image;
  gulong realized;
  gboolean icon_set;
} SDWindow;

static int64_t publish_icon(SDWindow *window) {
  GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(window->window));
  if (!surface)
    return SD_REQUESTED;
  GList *icons = NULL;
  for (SDPixels *pixels = window->image ? window->image->pixels : NULL; pixels;
       pixels = pixels->next) {
    GBytes *bytes =
        g_bytes_new(pixels->bytes, (gsize)pixels->width * pixels->height * 4);
    GdkTexture *texture = gdk_memory_texture_new(pixels->width, pixels->height,
                                                 GDK_MEMORY_R8G8B8A8, bytes,
                                                 (gsize)pixels->width * 4);
    g_bytes_unref(bytes);
    icons = g_list_append(icons, texture);
  }
  gdk_toplevel_set_icon_list(GDK_TOPLEVEL(surface), icons);
  g_list_free_full(icons, g_object_unref);
  return 0;
}
static void realized(GtkWidget *widget, gpointer data) {
  SDWindow *window = data;
  if (window->icon_set)
    publish_icon(window);
}
SDResult scoop_desktop_application_create(SDCallback callback, void *context,
                                          const uint8_t *id, int64_t length) {
  SDApplication *app = g_try_new0(SDApplication, 1);
  if (!app)
    return (SDResult){NULL, ENOMEM};
  app->callback = callback;
  app->context = context;
  return (SDResult){app, 0};
}
int32_t scoop_desktop_application_capabilities(void *app) { return 0; }
int32_t scoop_desktop_application_busy(void *app) { return 0; }
void scoop_desktop_application_cancel(void *app) { (void)app; }
void scoop_desktop_application_destroy(void *app) { g_free(app); }
int64_t scoop_desktop_application_icon(void *app, SDImage *image) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_application_badge(void *app, const uint8_t *text,
                                        int64_t length) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_application_menu(void *app, void *menu) {
  return SD_UNSUPPORTED;
}
SDResult scoop_desktop_window_create(void *app, void *native) {
  SDWindow *window = g_try_new0(SDWindow, 1);
  if (!window)
    return (SDResult){NULL, ENOMEM};
  window->window = GTK_WINDOW(native);
  window->realized =
      g_signal_connect(native, "realize", G_CALLBACK(realized), window);
  return (SDResult){window, 0};
}
int32_t scoop_desktop_window_capabilities(void *pointer) {
  SDWindow *window = pointer;
  return GDK_IS_X11_DISPLAY(gtk_widget_get_display(GTK_WIDGET(window->window)))
             ? 1
             : 0;
}
void scoop_desktop_window_destroy(void *pointer) {
  SDWindow *window = pointer;
  g_signal_handler_disconnect(window->window, window->realized);
  scoop_desktop_image_destroy(window->image);
  window->image = NULL;
  if (window->icon_set)
    publish_icon(window);
  g_free(window);
}
int64_t scoop_desktop_window_icon(void *pointer, SDImage *image) {
  SDWindow *window = pointer;
  if (!(scoop_desktop_window_capabilities(pointer) & 1))
    return SD_UNSUPPORTED;
  SDImage *previous = window->image;
  window->image = sd_retain_image(image);
  scoop_desktop_image_destroy(previous);
  window->icon_set = TRUE;
  return publish_icon(window);
}
int64_t scoop_desktop_window_overlay(void *window, SDImage *image,
                                     const uint8_t *text, int64_t length) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_window_progress(void *window, int32_t state,
                                      uint64_t completed, uint64_t total) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_window_caption(void *window, int32_t thumbnail,
                                     const uint8_t *text, int64_t length) {
  return SD_UNSUPPORTED;
}
