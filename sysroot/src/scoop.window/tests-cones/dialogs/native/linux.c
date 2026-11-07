#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
#include <gtk/gtk.h>
#include <stdint.h>
#include <string.h>

int32_t scoop_test_file_dialog(GtkWindow *owner, int32_t action, int32_t kind) {
  if (action == 4)
    return 1;
  if (action == 3)
    return gtk_widget_get_sensitive(GTK_WIDGET(owner));
  GListModel *windows = gtk_window_get_toplevels();
  int count = 0;
  for (guint index = 0; index < g_list_model_get_n_items(windows); ++index) {
    GtkWindow *window = g_list_model_get_item(windows, index);
    const char *title = gtk_window_get_title(window);
    if (!GTK_IS_FILE_CHOOSER(window) ||
        !gtk_widget_get_visible(GTK_WIDGET(window)) || !title ||
        strcmp(title, "Scoop file probe")) {
      g_object_unref(window);
      continue;
    }
    GtkFileChooser *chooser = GTK_FILE_CHOOSER(window);
    GtkFileChooserAction mode = gtk_file_chooser_get_action(chooser);
    int actual = mode == GTK_FILE_CHOOSER_ACTION_SAVE            ? 2
                 : mode == GTK_FILE_CHOOSER_ACTION_SELECT_FOLDER ? 3
                 : gtk_file_chooser_get_select_multiple(chooser) ? 1
                                                                 : 0;
    if ((kind >= 0 && kind != actual) ||
        (owner && gtk_window_get_transient_for(window) != owner)) {
      g_object_unref(window);
      continue;
    }
    ++count;
    if (action == 1) {
      gtk_dialog_response(GTK_DIALOG(window), GTK_RESPONSE_CANCEL);
      g_object_unref(window);
      return 1;
    }
    if (action == 2) {
      int result = gtk_window_get_modal(window);
      g_object_unref(window);
      return result;
    }
    g_object_unref(window);
  }
  return count;
}
int32_t scoop_test_file_choose(const uint8_t *data, int64_t length) {
  char *path = g_strndup((const char *)data, (gsize)length);
  GFile *expected = g_file_new_for_path(path);
  g_free(path);
  GListModel *windows = gtk_window_get_toplevels();
  int result = 0;
  for (guint index = 0; index < g_list_model_get_n_items(windows); ++index) {
    GtkWindow *window = g_list_model_get_item(windows, index);
    const char *title = gtk_window_get_title(window);
    if (!GTK_IS_FILE_CHOOSER(window) ||
        !gtk_widget_get_visible(GTK_WIDGET(window)) || !title ||
        strcmp(title, "Scoop file probe")) {
      g_object_unref(window);
      continue;
    }
    GtkFileChooser *chooser = GTK_FILE_CHOOSER(window);
    GFile *selected = gtk_file_chooser_get_file(chooser);
    if (selected && g_file_equal(selected, expected)) {
      gtk_dialog_response(GTK_DIALOG(window), GTK_RESPONSE_ACCEPT);
      result = 1;
    } else if (gtk_file_chooser_get_action(chooser) !=
               GTK_FILE_CHOOSER_ACTION_SAVE) {
      GError *error = NULL;
      gtk_file_chooser_set_file(chooser, expected, &error);
      g_clear_error(&error);
    }
    g_clear_object(&selected);
    g_object_unref(window);
    break;
  }
  g_object_unref(expected);
  return result;
}
#pragma clang diagnostic pop
