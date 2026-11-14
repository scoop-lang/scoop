#include "window.h"

typedef struct {
  bool done;
  GInputStream *stream;
  GByteArray *bytes;
  GError *error;
} SWClipboardRead;
static void read_bytes(GObject *object, GAsyncResult *result, gpointer data) {
  SWClipboardRead *read = data;
  GBytes *chunk =
      g_input_stream_read_bytes_finish(read->stream, result, &read->error);
  if (!chunk) {
    read->done = true;
    return;
  }
  gsize length;
  const void *bytes = g_bytes_get_data(chunk, &length);
  if (length > G_MAXUINT - read->bytes->len) {
    read->error = g_error_new_literal(G_IO_ERROR, G_IO_ERROR_NO_SPACE,
                                      "clipboard text is too large");
    read->done = true;
  } else if (length) {
    g_byte_array_append(read->bytes, bytes, length);
    g_input_stream_read_bytes_async(read->stream, 4096, G_PRIORITY_DEFAULT,
                                    NULL, read_bytes, read);
  } else {
    read->done = true;
  }
  g_bytes_unref(chunk);
}
static void clipboard_read(GObject *object, GAsyncResult *result,
                           gpointer data) {
  SWClipboardRead *read = data;
  read->stream = gdk_clipboard_read_finish(GDK_CLIPBOARD(object), result, NULL,
                                           &read->error);
  if (read->stream)
    g_input_stream_read_bytes_async(read->stream, 4096, G_PRIORITY_DEFAULT,
                                    NULL, read_bytes, read);
  else
    read->done = true;
}
SWClipboardText scoop_window_clipboard_read(const uint8_t *type_bytes,
                                            int64_t type_length) {
  SWApplication *app = sw_application;
  if (app->error)
    return (SWClipboardText){.error = -3};
  // Keep the byte length so the shared layer can reject embedded NUL.
  char *mime = strndup((const char *)type_bytes, type_length);
  if (!mime)
    return (SWClipboardText){.error = ENOMEM};
  bool plain = strcmp(mime, "text/plain") == 0;
  const char *formats[] = {plain ? "text/plain;charset=utf-8" : mime,
                           plain ? "text/plain" : NULL, NULL};
  SWClipboardRead read = {.bytes = g_byte_array_new()};
  gdk_clipboard_read_async(gdk_display_get_clipboard(app->display), formats,
                           G_PRIORITY_DEFAULT, NULL, clipboard_read, &read);
  // GTK signals enqueue events; owner handlers and borrowed fd watches wait.
  while (!read.done)
    g_main_context_iteration(app->main_context, TRUE);
  free(mime);
  gdk_gl_context_clear_current();
  g_clear_object(&read.stream);
  SWClipboardText result = {0};
  if (read.error) {
    bool absent =
        g_error_matches(read.error, G_IO_ERROR, G_IO_ERROR_NOT_SUPPORTED);
    result.error = absent ? 0 : EIO;
    g_clear_error(&read.error);
  } else {
    result.bytes = malloc(read.bytes->len ? read.bytes->len : 1);
    if (result.bytes) {
      result.length = read.bytes->len;
      if (result.length)
        memcpy(result.bytes, read.bytes->data, result.length);
    } else {
      result.error = ENOMEM;
    }
  }
  g_byte_array_unref(read.bytes);
  return result;
}
SWResult scoop_window_clipboard_create(void) {
  return (SWResult){g_ptr_array_new_with_free_func(g_object_unref), 0};
}

int64_t scoop_window_clipboard_add(void *writer, const uint8_t *type_bytes,
                                   int64_t type_length, const uint8_t *text,
                                   int64_t length) {
  char *mime = strndup((const char *)type_bytes, type_length);
  if (!mime)
    return ENOMEM;
  GdkContentProvider *provider;
  if (strcmp(mime, "text/plain") == 0) {
    char *copy = strndup((const char *)text, length);
    if (!copy) {
      free(mime);
      return ENOMEM;
    }
    provider = gdk_content_provider_new_typed(G_TYPE_STRING, copy);
    free(copy);
  } else {
    GBytes *bytes = g_bytes_new(text, length);
    provider = gdk_content_provider_new_for_bytes(mime, bytes);
    g_bytes_unref(bytes);
  }
  free(mime);
  if (!provider)
    return EIO;
  g_ptr_array_add(writer, provider);
  return 0;
}

int64_t scoop_window_clipboard_write(void *writer, SWApplication *app) {
  if (app->error)
    return -3;
  GPtrArray *providers = writer;
  // The union owns the references supplied to it; the writer keeps its own.
  for (guint index = 0; index < providers->len; ++index)
    g_object_ref(g_ptr_array_index(providers, index));
  GdkContentProvider *combined = gdk_content_provider_new_union(
      (GdkContentProvider **)providers->pdata, providers->len);
  gboolean written = gdk_clipboard_set_content(
      gdk_display_get_clipboard(app->display), combined);
  g_object_unref(combined);
  return written ? 0 : EIO;
}

void scoop_window_clipboard_destroy(void *writer) { g_ptr_array_unref(writer); }

void scoop_window_clipboard_release(uint8_t *bytes) { free(bytes); }
