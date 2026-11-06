#include "window.h"

GtkIMContext *sw_ime_context(SWWindow *window) {
  return window->ime_allowed ? window->ime : window->simple_ime;
}
static void commit(GtkIMContext *context, const char *text, gpointer data) {
  SWWindow *window = data;
  if (!window->destroying)
    sw_commit(window, text, strlen(text));
}
static void preedit(GtkIMContext *context, gpointer data) {
  SWWindow *window = data;
  char *text = NULL;
  PangoAttrList *attributes = NULL;
  int cursor;
  gtk_im_context_get_preedit_string(context, &text, &attributes, &cursor);
  size_t length = strlen(text);
  int64_t position = sw_utf8_offset(text, length, cursor);
  sw_preedit(window, text, length, position, position);
  if (attributes)
    pango_attr_list_unref(attributes);
  g_free(text);
}
static gboolean surrounding(GtkIMContext *context, gpointer data) {
  SWWindow *window = data;
  SWTextSnapshotData *document = &window->document;
  if (!window->ime_allowed || document->revision < 0 ||
      document->connection != window->connection ||
      document->after != window->sequence)
    return FALSE;
  gtk_im_context_set_surrounding_with_selection(
      context, (const char *)document->text, (int)document->length,
      (int)document->cursor, (int)document->anchor);
  return TRUE;
}
static gboolean delete_surrounding(GtkIMContext *context, int offset, int count,
                                   gpointer data) {
  SWWindow *window = data;
  SWTextSnapshotData *document = &window->document;
  if (!window->ime_allowed || document->revision < 0 ||
      document->connection != window->connection ||
      document->after != window->sequence || count < 0)
    return FALSE;
  const char *text = (const char *)document->text;
  int64_t cursor = g_utf8_pointer_to_offset(text, text + document->cursor);
  int64_t start = cursor + offset, end = start + count;
  if (start < 0 || end > g_utf8_strlen(text, document->length))
    return FALSE;
  int64_t begin = g_utf8_offset_to_pointer(text, start) - text;
  int64_t finish = g_utf8_offset_to_pointer(text, end) - text;
  SWEvent *event = sw_text_batch(window);
  if (!sw_text_append(event, 1, "", 0, -1, -1)) {
    sw_free_event(event);
    sw_fail(window->application, ENOMEM);
    return FALSE;
  }
  event->data.flags |= 1;
  event->data.b = document->revision;
  event->edits[0].has_replacement = true;
  event->edits[0].replacement_start = begin;
  event->edits[0].replacement_end = finish;
  sw_text_deliver(window, event);
  return TRUE;
}
void sw_ime_install(SWWindow *window) {
  window->ime = gtk_im_multicontext_new();
  window->simple_ime = gtk_im_context_simple_new();
  GtkIMContext *contexts[] = {window->ime, window->simple_ime};
  for (size_t index = 0; index < G_N_ELEMENTS(contexts); ++index) {
    GtkIMContext *context = contexts[index];
    gtk_im_context_set_client_widget(context, window->view);
    g_signal_connect(context, "commit", G_CALLBACK(commit), window);
    g_signal_connect(context, "preedit-changed", G_CALLBACK(preedit), window);
    g_signal_connect(context, "retrieve-surrounding", G_CALLBACK(surrounding),
                     window);
    g_signal_connect(context, "delete-surrounding",
                     G_CALLBACK(delete_surrounding), window);
  }
}
void sw_ime_update(SWWindow *window) {
  GtkIMContext *current = sw_ime_context(window);
  gtk_im_context_focus_out(current == window->ime ? window->simple_ime
                                                  : window->ime);
  if (window->snapshot.focused && gtk_widget_has_focus(window->view))
    gtk_im_context_focus_in(current);
  else
    gtk_im_context_focus_out(current);
  sw_ime_document(window);
  sw_ime_caret(window);
}
void sw_ime_caret(SWWindow *window) {
  GdkRectangle rectangle = {0};
  if (window->caret_known) {
    rectangle.x = (int)floor(window->caret_x);
    rectangle.y = (int)floor(window->caret_y);
    rectangle.width = (int)ceil(window->caret_width);
    rectangle.height = (int)ceil(window->caret_height);
  }
  gtk_im_context_set_cursor_location(sw_ime_context(window), &rectangle);
}
void sw_ime_document(SWWindow *window) {
  surrounding(sw_ime_context(window), window);
}
