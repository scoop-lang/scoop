#include "drag.h"

typedef struct SWDropRead SWDropRead;
typedef struct {
  unsigned references;
  SWWindow *owner;
  GtkDropTargetAsync *controller;
  SWDropCallback callback;
  void *context;
  GdkDrop *drop;
  GSource *timer;
  SWDropRead *read;
  uint64_t revision;
  double x, y;
  int choice;
  bool enabled, closed, inside, in_callback, periodic;
} SWDropTarget;

struct SWDropRead {
  SWDropTarget *target;
  GdkDrop *drop;
  GInputStream *stream;
  GCancellable *cancel;
  GByteArray *bytes;
  uint64_t revision;
  int choice;
  bool finished, committing;
};

static SWDropTarget *retain(SWDropTarget *target) {
  ++target->references;
  return target;
}
static void release(gpointer data) {
  SWDropTarget *target = data;
  if (--target->references)
    return;
  g_clear_object(&target->drop);
  g_clear_object(&target->controller);
  free(target);
}
static bool available(SWDropTarget *target) {
  return target->owner && !target->closed && target->enabled &&
         target->callback && !target->owner->destroying &&
         !target->owner->input_blocked &&
         gtk_widget_get_visible(target->owner->window) &&
         !target->owner->ignore_mouse;
}
static int formats(GdkDrop *drop) {
  GdkContentFormats *values = gdk_drop_get_formats(drop);
  bool text = gdk_content_formats_contain_mime_type(
                  values, "text/plain;charset=utf-8") ||
              gdk_content_formats_contain_mime_type(values, "text/plain") ||
              gdk_content_formats_contain_gtype(values, G_TYPE_STRING);
  bool uris = gdk_content_formats_contain_mime_type(values, "text/uri-list");
  return (text ? 1 : 0) | (uris ? 6 : 0);
}
static int allowed(SWDropTarget *target) {
  return target->drop && target->owner
             ? sw_drag_current_effects(target->owner, target->drop,
                                       gdk_drop_get_actions(target->drop) & 7)
             : 0;
}
static int call(SWDropTarget *target, int phase, int choice,
                GPtrArray *payload) {
  if (target->closed || !target->callback || target->in_callback)
    return 0;
  int offered = target->drop ? formats(target->drop) : 0;
  int effects = allowed(target);
  int local = target->drop && target->owner
                  ? sw_drag_local(target->owner, target->drop)
                  : 0;
  uint64_t modifiers = 0;
  if (target->owner) {
    GdkModifierType state = 0;
    GdkDevice *device = target->drop ? gdk_drop_get_device(target->drop) : NULL;
    if (device) {
      double x, y;
      gdk_surface_get_device_position(gdk_drop_get_surface(target->drop),
                                      device, &x, &y, &state);
      target->owner->application->input.gdk_modifiers = state;
    }
    modifiers = sw_modifiers(target->owner->application);
  }
  target->in_callback = true;
  int result =
      target->callback(target->context, phase, offered, effects, local,
                       target->x, target->y, modifiers, choice, payload);
  target->in_callback = false;
  return result;
}
static int query(SWDropTarget *target) {
  if (!target->inside || !target->drop || target->read || !available(target) ||
      target->in_callback)
    return 0;
  uint64_t revision = target->revision;
  int choice = call(target, 0, 0, NULL);
  return target->inside && target->drop && revision == target->revision &&
                 available(target)
             ? sw_drop_choice(choice, formats(target->drop), allowed(target))
             : 0;
}
static void stop_timer(SWDropTarget *target) {
  if (target->timer) {
    GSource *timer = target->timer;
    target->timer = NULL;
    g_source_destroy(timer);
    g_source_unref(timer);
  }
}
static void finish(SWDropTarget *target) {
  bool notify = target->inside;
  target->inside = false;
  target->choice = 0;
  stop_timer(target);
  if (notify)
    call(target, 2, 0, NULL);
  g_clear_object(&target->drop);
}
static gboolean tick(gpointer data) {
  SWDropTarget *target = retain(data);
  if (!target->periodic || !target->inside || target->read ||
      !available(target)) {
    stop_timer(target);
    release(target);
    return G_SOURCE_REMOVE;
  }
  target->choice = query(target);
  if (target->drop && available(target))
    gdk_drop_status(target->drop, (GdkDragAction)allowed(target),
                    (GdkDragAction)(target->choice & 255));
  bool keep = target->timer != NULL;
  release(target);
  return keep ? G_SOURCE_CONTINUE : G_SOURCE_REMOVE;
}
static void update_timer(SWDropTarget *target) {
  stop_timer(target);
  if (!target->periodic || !target->inside || target->read ||
      !available(target))
    return;
  target->timer = g_timeout_source_new(16);
  g_source_set_callback(target->timer, tick, retain(target), release);
  g_source_attach(target->timer, target->owner->application->main_context);
}
static gboolean accept_drop(GtkDropTargetAsync *controller, GdkDrop *drop,
                            gpointer data) {
  (void)controller;
  SWDropTarget *target = data;
  return available(target) && formats(drop) != 0;
}
static GdkDragAction motion(GtkDropTargetAsync *controller, GdkDrop *drop,
                            double x, double y, gpointer data) {
  (void)controller;
  SWDropTarget *target = retain(data);
  int choice = 0;
  if (!target->in_callback && !target->read && available(target) &&
      isfinite(x) && isfinite(y)) {
    if (target->drop != drop) {
      finish(target);
      if (available(target))
        target->drop = g_object_ref(drop);
    }
    if (available(target)) {
      target->inside = true;
      target->x = x;
      target->y = y;
      choice = target->choice = query(target);
      update_timer(target);
    }
  }
  release(target);
  return (GdkDragAction)(choice & 255);
}
static void leave(GtkDropTargetAsync *controller, GdkDrop *drop,
                  gpointer data) {
  (void)controller;
  SWDropTarget *target = retain(data);
  // GTK may leave the widget immediately after accepting Drop. The async
  // transfer owns that drop until it reports an actual destination result.
  if (!target->read && !target->in_callback && target->drop == drop)
    finish(target);
  release(target);
}

static void complete_read(SWDropRead *read, bool success) {
  SWDropTarget *target = read->target;
  bool accepted = false;
  if (success && !read->finished && target->read == read &&
      read->revision == target->revision && target->drop == read->drop &&
      available(target)) {
    target->read = NULL;
    int choice = query(target);
    target->read = read;
    if (choice == read->choice && read->revision == target->revision &&
        available(target)) {
      GPtrArray *payload =
          sw_drop_decode(read->bytes->data, read->bytes->len, choice / 256);
      if (payload) {
        read->committing = true;
        accepted = call(target, 1, choice, payload) != 0;
        read->committing = false;
        g_ptr_array_unref(payload);
      }
    }
  }
  if (target->read == read) {
    target->read = NULL;
    finish(target);
  }
  if (!read->finished && !read->committing) {
    read->finished = true;
    gdk_drop_finish(read->drop,
                    accepted ? (GdkDragAction)(read->choice & 255) : 0);
  }
  if (read->stream)
    g_input_stream_close_async(read->stream, G_PRIORITY_DEFAULT, NULL, NULL,
                               NULL);
  g_clear_object(&read->stream);
  g_object_unref(read->cancel);
  g_object_unref(read->drop);
  g_byte_array_unref(read->bytes);
  release(target);
  free(read);
}
static void read_chunk(GObject *object, GAsyncResult *result, gpointer data) {
  SWDropRead *read = data;
  GError *error = NULL;
  GBytes *chunk =
      g_input_stream_read_bytes_finish(G_INPUT_STREAM(object), result, &error);
  if (!chunk) {
    g_clear_error(&error);
    complete_read(read, false);
    return;
  }
  gsize length;
  const uint8_t *bytes = g_bytes_get_data(chunk, &length);
  bool valid = length <= G_MAXUINT - read->bytes->len && !read->finished &&
               !g_cancellable_is_cancelled(read->cancel);
  if (valid && length)
    g_byte_array_append(read->bytes, bytes, length);
  g_bytes_unref(chunk);
  if (!valid || !length) {
    complete_read(read, valid);
    return;
  }
  g_input_stream_read_bytes_async(read->stream, 4096, G_PRIORITY_DEFAULT,
                                  read->cancel, read_chunk, read);
}
static void opened(GObject *object, GAsyncResult *result, gpointer data) {
  SWDropRead *read = data;
  GError *error = NULL;
  read->stream = gdk_drop_read_finish(GDK_DROP(object), result, NULL, &error);
  if (!read->stream || read->finished ||
      g_cancellable_is_cancelled(read->cancel)) {
    g_clear_error(&error);
    complete_read(read, false);
    return;
  }
  g_input_stream_read_bytes_async(read->stream, 4096, G_PRIORITY_DEFAULT,
                                  read->cancel, read_chunk, read);
}
static gboolean drop(GtkDropTargetAsync *controller, GdkDrop *value, double x,
                     double y, gpointer data) {
  SWDropTarget *target = retain(data);
  motion(controller, value, x, y, target);
  int choice = target->choice;
  if (!choice || !available(target) || target->read || target->in_callback) {
    release(target);
    return FALSE;
  }
  SWDropRead *read = calloc(1, sizeof(*read));
  if (!read) {
    release(target);
    return FALSE;
  }
  read->target = target;
  read->drop = g_object_ref(value);
  read->cancel = g_cancellable_new();
  read->bytes = g_byte_array_new();
  read->revision = target->revision;
  read->choice = choice;
  target->read = read;
  stop_timer(target);
  static const char *text_types[] = {"text/plain;charset=utf-8", "text/plain",
                                     NULL};
  static const char *uri_types[] = {"text/uri-list", NULL};
  GMainContext *context = target->owner->application->main_context;
  g_main_context_push_thread_default(context);
  gdk_drop_read_async(value, choice / 256 == 1 ? text_types : uri_types,
                      G_PRIORITY_DEFAULT, read->cancel, opened, read);
  g_main_context_pop_thread_default(context);
  return TRUE;
}
static void cancel_read(SWDropTarget *target) {
  SWDropRead *read = target->read;
  if (!read)
    return;
  g_cancellable_cancel(read->cancel);
  if (!read->finished && !read->committing) {
    read->finished = true;
    gdk_drop_finish(read->drop, 0);
  }
}
static void close_target(SWDropTarget *target) {
  if (target->closed)
    return;
  target->closed = true;
  target->callback = NULL;
  target->context = NULL;
  ++target->revision;
  cancel_read(target);
  finish(target);
  if (target->owner) {
    target->owner->drop_target = NULL;
    gtk_widget_remove_controller(target->owner->view,
                                 GTK_EVENT_CONTROLLER(target->controller));
    target->owner = NULL;
  }
  g_signal_handlers_disconnect_by_data(target->controller, target);
}

SWResult scoop_window_drop_create(SWWindow *window, SWDropCallback callback,
                                  void *context) {
  if (!callback || window->destroying)
    return (SWResult){NULL, -4};
  if (window->drop_target)
    return (SWResult){NULL, -5};
  SWDropTarget *target = calloc(1, sizeof(*target));
  if (!target)
    return (SWResult){NULL, ENOMEM};
  target->references = 1;
  target->owner = window;
  target->callback = callback;
  target->context = context;
  target->enabled = true;
  const char *types[] = {"text/plain;charset=utf-8", "text/plain",
                         "text/uri-list"};
  GdkContentFormats *content =
      gdk_content_formats_new(types, G_N_ELEMENTS(types));
  target->controller = gtk_drop_target_async_new(
      content, GDK_ACTION_COPY | GDK_ACTION_MOVE | GDK_ACTION_LINK);
  g_signal_connect(target->controller, "accept", G_CALLBACK(accept_drop),
                   target);
  g_signal_connect(target->controller, "drag-enter", G_CALLBACK(motion),
                   target);
  g_signal_connect(target->controller, "drag-motion", G_CALLBACK(motion),
                   target);
  g_signal_connect(target->controller, "drag-leave", G_CALLBACK(leave), target);
  g_signal_connect(target->controller, "drop", G_CALLBACK(drop), target);
  gtk_widget_add_controller(
      window->view, g_object_ref(GTK_EVENT_CONTROLLER(target->controller)));
  window->drop_target = target;
  return (SWResult){target, 0};
}
void scoop_window_drop_update(SWDropTarget *target, int32_t enabled) {
  ++target->revision;
  target->enabled = enabled != 0;
  target->choice = 0;
  cancel_read(target);
  update_timer(target);
}
void scoop_window_drop_periodic(SWDropTarget *target, int32_t enabled) {
  target->periodic = enabled != 0;
  update_timer(target);
}
void scoop_window_drop_leave(SWDropTarget *target) {
  retain(target);
  cancel_read(target);
  finish(target);
  release(target);
}
void scoop_window_drop_close(SWDropTarget *target) {
  close_target(target);
  release(target);
}
void sw_drop_window_destroy(SWWindow *window) {
  if (window->drop_target)
    close_target(window->drop_target);
}
