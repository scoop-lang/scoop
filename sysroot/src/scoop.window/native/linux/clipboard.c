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
SWClipboardText scoop_window_clipboard_read(void) {
  SWApplication *app = sw_application;
  if (app->error)
    return (SWClipboardText){.error = -3};
  // Keep the byte length so the shared layer can reject embedded NUL.
  static const char *formats[] = {"text/plain;charset=utf-8", "text/plain",
                                  NULL};
  SWClipboardRead read = {.bytes = g_byte_array_new()};
  gdk_clipboard_read_async(gdk_display_get_clipboard(app->display), formats,
                           G_PRIORITY_DEFAULT, NULL, clipboard_read, &read);
  // GTK signals enqueue events; owner handlers and borrowed fd watches wait.
  while (!read.done)
    g_main_context_iteration(app->main_context, TRUE);
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
int64_t scoop_window_clipboard_write(const uint8_t *text, int64_t length) {
  SWApplication *app = sw_application;
  if (app->error)
    return -3;
  char *copy = strndup((const char *)text, length);
  if (!copy)
    return ENOMEM;
  gdk_clipboard_set_text(gdk_display_get_clipboard(app->display), copy);
  free(copy);
  return 0;
}
void scoop_window_clipboard_release(uint8_t *bytes) { free(bytes); }
