#include "drag.h"
#include "input/delivery.h"
#include <limits.h>
#include <math.h>

typedef struct {
  unsigned references;
  SWWindow *owner;
  SWApplication *application;
  SWDragCallback callback;
  void *context;
  GdkEvent *event;
  GdkDrag *drag;
  GdkPaintable *preview;
  int hot_x, hot_y;
  GPtrArray *providers;
  GString *uris;
  int format, effects, initial_effects;
  int64_t contact;
  bool direct, ready, active, closed;
} SWDragSource;

static SWDragSource *retain(SWDragSource *source) {
  ++source->references;
  return source;
}
static void release(SWDragSource *source) {
  if (--source->references)
    return;
  if (source->event)
    gdk_event_unref(source->event);
  g_clear_object(&source->drag);
  g_clear_object(&source->preview);
  g_ptr_array_unref(source->providers);
  g_string_free(source->uris, TRUE);
  free(source);
}
static bool available(SWDragSource *source) {
  SWWindow *window = source->owner;
  return window && !window->destroying && !window->input_blocked &&
         gtk_widget_get_visible(window->window) && !window->ignore_mouse;
}
static void detach(SWDragSource *source) {
  if (source->application && source->application->drag_source == source)
    source->application->drag_source = NULL;
  source->application = NULL;
  source->owner = NULL;
}
static void finish(SWDragSource *source, int effect) {
  if (!source->active)
    return;
  source->active = false;
  GdkDrag *drag = source->drag;
  source->drag = NULL;
  g_signal_handlers_disconnect_by_data(drag, source);
  detach(source);
  if (!(source->initial_effects & effect))
    effect = 0;
  if (source->callback)
    source->callback(source->context, 1, effect);
  gdk_drag_drop_done(drag, effect != 0);
  g_object_unref(drag);
  release(source);
}
static void finished(GdkDrag *drag, gpointer data) {
  int effect = gdk_drag_get_selected_action(drag) & 7;
  finish(data, effect == 1 || effect == 2 || effect == 4 ? effect : 0);
}
static void cancelled(GdkDrag *drag, GdkDragCancelReason reason,
                      gpointer data) {
  (void)drag;
  (void)reason;
  finish(data, 0);
}
static void add_bytes(SWDragSource *source, const char *mime,
                      const uint8_t *bytes, size_t length) {
  GBytes *value = g_bytes_new(bytes, length);
  g_ptr_array_add(source->providers,
                  gdk_content_provider_new_for_bytes(mime, value));
  g_bytes_unref(value);
}

SWResult scoop_window_drag_create(SWWindow *window, SWNativeInput *input,
                                  int32_t effects, SWDragCallback callback,
                                  void *context) {
  if (!callback || effects <= 0 || effects > 7)
    return (SWResult){NULL, -8};
  if (!input || input->window != window->identity || !input->event)
    return (SWResult){NULL, -4};
  GdkEventType type = gdk_event_get_event_type(input->event);
  bool direct = input->kind == SWContact;
  bool live =
      direct
          ? sw_contact_find(window->input.contacts, input->contact) != NULL
          : ((type == GDK_BUTTON_PRESS &&
              gdk_button_event_get_button(input->event) == 1) ||
             (type == GDK_MOTION_NOTIFY &&
              (gdk_event_get_modifier_state(input->event) & GDK_BUTTON1_MASK)));
  if (!live ||
      (direct && (!window->input.contacts || window->input.contacts->next)) ||
      gdk_event_get_surface(input->event) != sw_surface(window))
    return (SWResult){NULL, -4};
  if (window->application->drag_source)
    return (SWResult){NULL, -5};
  SWDragSource *source = calloc(1, sizeof(*source));
  if (!source)
    return (SWResult){NULL, ENOMEM};
  source->references = 1;
  source->owner = window;
  source->application = window->application;
  source->callback = callback;
  source->context = context;
  source->event = gdk_event_ref(input->event);
  source->providers = g_ptr_array_new_with_free_func(g_object_unref);
  source->uris = g_string_new(NULL);
  source->effects = source->initial_effects = effects;
  source->direct = direct;
  source->contact = input->contact;
  if (!available(source)) {
    release(source);
    return (SWResult){NULL, -4};
  }
  return (SWResult){source, 0};
}
int64_t scoop_window_drag_add(SWDragSource *source, int32_t format,
                              const uint8_t *bytes, int64_t length) {
  if (source->closed || source->ready || source->active)
    return -4;
  if (source->format || (format != 1 && format != 2) || length < 0 ||
      (length &&
       (!bytes || !g_utf8_validate((const char *)bytes, length, NULL))))
    return -8;
  if (format == 2) {
    if (!length)
      return -8;
    char *uri = g_strndup((const char *)bytes, length);
    bool valid =
        sw_drag_absolute_uri(uri) && !strchr(uri, '\r') && !strchr(uri, '\n');
    if (valid) {
      g_string_append(source->uris, uri);
      g_string_append(source->uris, "\r\n");
    }
    g_free(uri);
    if (!valid)
      return -8;
  }
  add_bytes(source, "text/plain;charset=utf-8", bytes, length);
  add_bytes(source, "text/plain", bytes, length);
  source->format = format;
  return 0;
}
int64_t scoop_window_drag_add_path(SWDragSource *source, const uint8_t *bytes,
                                   int64_t length) {
  if (source->closed || source->ready || source->active)
    return -4;
  if ((source->format && source->format != 3) || !bytes || length <= 0 ||
      memchr(bytes, 0, length) || bytes[0] != '/')
    return -8;
  char *path = g_strndup((const char *)bytes, length);
  GFile *file = g_file_new_for_path(path);
  char *uri = g_file_get_uri(file);
  g_object_unref(file);
  g_free(path);
  if (!uri)
    return -8;
  g_string_append(source->uris, uri);
  g_string_append(source->uris, "\r\n");
  g_free(uri);
  source->format = 3;
  return 0;
}
int64_t scoop_window_drag_preview(SWDragSource *source, int32_t width,
                                  int32_t height, const uint8_t *pixels,
                                  double logical_width, double logical_height,
                                  double hot_x, double hot_y) {
  if (fabs(hot_x) > INT_MAX || fabs(hot_y) > INT_MAX ||
      logical_width > INT_MAX || logical_height > INT_MAX)
    return -8;
  GBytes *bytes = g_bytes_new(pixels, (gsize)width * height * 4);
  GdkTexture *texture = gdk_memory_texture_new(
      width, height, GDK_MEMORY_R8G8B8A8, bytes, (gsize)width * 4);
  g_bytes_unref(bytes);
  // A texture's intrinsic size is in pixels; the icon uses logical units.
  GtkSnapshot *snapshot = gtk_snapshot_new();
  graphene_rect_t bounds =
      GRAPHENE_RECT_INIT(0, 0, (float)logical_width, (float)logical_height);
  gtk_snapshot_append_texture(snapshot, texture, &bounds);
  g_object_unref(texture);
  source->preview = gtk_snapshot_free_to_paintable(snapshot, &bounds.size);
  source->hot_x = (int)round(hot_x);
  source->hot_y = (int)round(hot_y);
  return 0;
}

int64_t scoop_window_drag_ready(SWDragSource *source) {
  if (source->closed || source->ready || source->active || !available(source))
    return -4;
  if (source->application->drag_source)
    return -5;
  if (!source->format)
    return -8;
  if (source->uris->len)
    add_bytes(source, "text/uri-list", (const uint8_t *)source->uris->str,
              source->uris->len);
  source->ready = true;
  source->application->drag_source = source;
  return 0;
}
int64_t scoop_window_drag_start(SWDragSource *source) {
  if (source->closed || !source->ready || source->active || !available(source))
    return -4;
  if (source->direct &&
      (!sw_contact_find(source->owner->input.contacts, source->contact) ||
       source->owner->input.contacts->next)) {
    source->ready = false;
    detach(source);
    if (source->callback)
      source->callback(source->context, 1, 0);
    return 0;
  }
  GdkContentProvider **providers =
      g_new(GdkContentProvider *, source->providers->len);
  for (guint n = 0; n < source->providers->len; ++n)
    providers[n] = g_object_ref(g_ptr_array_index(source->providers, n));
  GdkContentProvider *content =
      gdk_content_provider_new_union(providers, source->providers->len);
  g_free(providers);
  source->drag = gdk_drag_begin(gdk_event_get_surface(source->event),
                                gdk_event_get_device(source->event), content,
                                (GdkDragAction)source->effects, 0, 0);
  g_object_unref(content);
  if (!source->drag)
    return -2;
  source->ready = false;
  source->active = true;
  retain(source);
  g_signal_connect(source->drag, "dnd-finished", G_CALLBACK(finished), source);
  g_signal_connect(source->drag, "cancel", G_CALLBACK(cancelled), source);
  if (source->preview) {
    gtk_drag_icon_set_from_paintable(source->drag, source->preview,
                                     source->hot_x, source->hot_y);
  } else {
    GtkWidget *icon = gtk_drag_icon_get_for_drag(source->drag);
    gtk_drag_icon_set_child(GTK_DRAG_ICON(icon),
                            gtk_image_new_from_icon_name("text-x-generic"));
    gdk_drag_set_hotspot(source->drag, 12, 12);
  }
  if (source->callback)
    source->callback(source->context, 0, 0);
  return 0;
}
int64_t scoop_window_drag_effects(SWDragSource *source, int32_t effects) {
  if (source->closed || (!source->ready && !source->active))
    return -4;
  if (effects < 0 || (effects & ~source->initial_effects))
    return -8;
  source->effects = effects;
  if (source->drag)
    g_object_set(source->drag, "actions", (GdkDragAction)effects, NULL);
  return 0;
}
void scoop_window_drag_close(SWDragSource *source) {
  source->closed = true;
  source->callback = NULL;
  source->context = NULL;
  detach(source);
  release(source);
}
int sw_drag_local(SWWindow *window, GdkDrop *drop) {
  GdkDrag *drag = gdk_drop_get_drag(drop);
  SWDragSource *source = window->application->drag_source;
  return (drag ? 1 : 0) |
         (drag && source && source->drag == drag && !source->closed ? 2 : 0);
}
int sw_drag_current_effects(SWWindow *window, GdkDrop *drop, int fallback) {
  SWDragSource *source = window->application->drag_source;
  return source && source->drag == gdk_drop_get_drag(drop)
             ? fallback & source->effects
             : fallback;
}
void sw_drag_window_destroy(SWWindow *window) {
  SWDragSource *source = window->application->drag_source;
  if (source && source->owner == window)
    detach(source);
}
