#include "../dialogs.h"
#include "window.h"

typedef struct {
  SWApplication *application;
  SWWindow *owner;
  GtkFileDialog *dialog;
  GCancellable *cancel;
  GtkFileFilter *filter;
  GPtrArray *paths;
  char *domain;
  int kind, outcome;
  int64_t error;
  bool started, cancelled;
} SWFileDialog;

SWResult scoop_window_file_create(SWApplication *application, SWWindow *owner,
                                  int32_t kind) {
  SWFileDialog *session = calloc(1, sizeof(*session));
  if (!session)
    return (SWResult){NULL, ENOMEM};
  session->application = application;
  session->owner = owner;
  session->kind = kind;
  session->dialog = gtk_file_dialog_new();
  gtk_file_dialog_set_modal(session->dialog, TRUE);
  session->cancel = g_cancellable_new();
  session->paths = g_ptr_array_new_with_free_func(g_free);
  return (SWResult){session, 0};
}

int64_t scoop_window_file_option(SWFileDialog *session, int32_t field,
                                 const uint8_t *data, int64_t length) {
  char *text = g_strndup((const char *)data, (gsize)length);
  int64_t result = 0;
  if (!field)
    gtk_file_dialog_set_title(session->dialog, text);
  else if (field == 1) {
    GFile *directory = g_file_new_for_path(text);
    gtk_file_dialog_set_initial_folder(session->dialog, directory);
    g_object_unref(directory);
  } else if (field == 2) {
    char *name = g_filename_to_utf8(text, -1, NULL, NULL, NULL);
    if (name) {
      gtk_file_dialog_set_initial_name(session->dialog, name);
      g_free(name);
    } else
      result = -1;
  } else if (field == 3) {
    if (!session->filter) {
      session->filter = gtk_file_filter_new();
      GListStore *filters = g_list_store_new(GTK_TYPE_FILE_FILTER);
      g_list_store_append(filters, session->filter);
      gtk_file_dialog_set_filters(session->dialog, G_LIST_MODEL(filters));
      gtk_file_dialog_set_default_filter(session->dialog, session->filter);
      g_object_unref(filters);
    }
    const char *previous = gtk_file_filter_get_name(session->filter);
    char *name = previous ? g_strdup_printf("%s, *.%s", previous, text)
                          : g_strdup_printf("*.%s", text);
    gtk_file_filter_set_name(session->filter, name);
    gtk_file_filter_add_suffix(session->filter, text);
    g_free(name);
  } else
    result = -4;
  g_free(text);
  return result;
}

static void add_path(SWFileDialog *session, GFile *file) {
  char *path = g_file_get_path(file);
  if (path)
    g_ptr_array_add(session->paths, path);
  else {
    session->outcome = 3;
    session->error = -1;
  }
}

static void finished(GObject *source, GAsyncResult *result, gpointer context) {
  SWFileDialog *session = context;
  GError *error = NULL;
  if (session->kind == 1) {
    GListModel *files =
        gtk_file_dialog_open_multiple_finish(session->dialog, result, &error);
    if (files) {
      for (guint index = 0; index < g_list_model_get_n_items(files); ++index) {
        GFile *file = g_list_model_get_item(files, index);
        add_path(session, file);
        g_object_unref(file);
      }
      g_object_unref(files);
    }
  } else {
    GFile *file =
        session->kind == 0
            ? gtk_file_dialog_open_finish(session->dialog, result, &error)
        : session->kind == 2
            ? gtk_file_dialog_save_finish(session->dialog, result, &error)
            : gtk_file_dialog_select_folder_finish(session->dialog, result,
                                                   &error);
    if (file) {
      add_path(session, file);
      g_object_unref(file);
    }
  }
  if (error) {
    if (g_error_matches(error, GTK_DIALOG_ERROR, GTK_DIALOG_ERROR_DISMISSED) ||
        g_error_matches(error, GTK_DIALOG_ERROR, GTK_DIALOG_ERROR_CANCELLED) ||
        g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED))
      session->outcome = 2;
    else {
      session->outcome = 3;
      session->error = error->code;
      session->domain = g_strdup(g_quark_to_string(error->domain));
    }
    g_error_free(error);
  } else if (!session->outcome) {
    if (session->paths->len)
      session->outcome = 1;
    else {
      session->outcome = 3;
      session->error = EIO;
      session->domain = g_strdup("GtkFileDialog");
    }
  }
  sw_wake(session->application);
}

void scoop_window_file_start(SWFileDialog *session) {
  if (session->outcome)
    return;
  session->started = true;
  GtkWindow *parent =
      session->owner ? GTK_WINDOW(session->owner->window) : NULL;
  switch (session->kind) {
  case 0:
    gtk_file_dialog_open(session->dialog, parent, session->cancel, finished,
                         session);
    break;
  case 1:
    gtk_file_dialog_open_multiple(session->dialog, parent, session->cancel,
                                  finished, session);
    break;
  case 2:
    gtk_file_dialog_save(session->dialog, parent, session->cancel, finished,
                         session);
    break;
  case 3:
    gtk_file_dialog_select_folder(session->dialog, parent, session->cancel,
                                  finished, session);
    break;
  default:
    session->error = EINVAL;
    session->outcome = 3;
    sw_wake(session->application);
  }
}
int32_t scoop_window_file_cancel(SWFileDialog *session) {
  if (session->outcome || session->cancelled)
    return 0;
  session->cancelled = true;
  if (session->started)
    g_cancellable_cancel(session->cancel);
  else
    session->outcome = 2;
  sw_wake(session->application);
  return 1;
}
SWFileResult scoop_window_file_result(SWFileDialog *session) {
  return (SWFileResult){session->outcome, session->error, session->paths->len,
                        (const uint8_t *)session->domain,
                        session->domain ? (int64_t)strlen(session->domain) : 0};
}
SWFilePath scoop_window_file_path(SWFileDialog *session, int64_t index) {
  const char *path = g_ptr_array_index(session->paths, (guint)index);
  return (SWFilePath){(const uint8_t *)path, (int64_t)strlen(path)};
}
void scoop_window_file_release(SWFileDialog *session) {
  g_object_unref(session->dialog);
  g_object_unref(session->cancel);
  g_clear_object(&session->filter);
  g_ptr_array_unref(session->paths);
  g_free(session->domain);
  free(session);
}
