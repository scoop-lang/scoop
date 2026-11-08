#include "window.h"
#include <gdk/wayland/gdkwayland.h>
#include <gdk/x11/gdkx.h>
#include <stdatomic.h>
#include <sys/eventfd.h>
#include <sys/syscall.h>

SWApplication *sw_application;
static atomic_int selected_backend = SW_X11;

double sw_now(void) {
  struct timespec value;
  clock_gettime(CLOCK_MONOTONIC, &value);
  return (double)value.tv_sec + (double)value.tv_nsec / 1000000000.0;
}
void sw_wake(SWApplication *app) {
  uint64_t value = 1;
  ssize_t result;
  do {
    result = write(app->wake_fd, &value, sizeof(value));
  } while (result < 0 && errno == EINTR);
}
void sw_fail(SWApplication *app, int error) {
  if (!app->error)
    app->error = error ? error : EIO;
  sw_wake(app);
}
int32_t scoop_window_is_main_thread(void) {
  return syscall(SYS_gettid) == getpid();
}
int32_t scoop_window_linux_backend(void) {
  return atomic_load(&selected_backend);
}
static void display_closed(GdkDisplay *display, gboolean error, gpointer data) {
  sw_fail(data, EPIPE);
}
static void finish(SWApplication *app) {
  if (app->display_closed)
    g_signal_handler_disconnect(app->display, app->display_closed);
  if (app->monitors_changed)
    g_signal_handler_disconnect(app->monitors, app->monitors_changed);
  if (app->theme_changed)
    g_signal_handler_disconnect(adw_style_manager_get_default(),
                                app->theme_changed);
  g_clear_object(&app->display);
  if (app->main_context) {
    g_main_context_release(app->main_context);
    g_main_context_unref(app->main_context);
  }
  if (app->wake_fd >= 0)
    close(app->wake_fd);
  while (app->head) {
    SWEvent *event = app->head;
    app->head = event->next;
    sw_free_event(event);
  }
  g_free(app->application_id);
  free(app);
}
SWResult scoop_window_application_create(SWCallback callback, void *context,
                                         int32_t policy,
                                         const uint8_t *identifier,
                                         int64_t length) {
  if (length > 0) {
    char *id = g_strndup((const char *)identifier, (gsize)length);
    if (!g_application_id_is_valid(id)) {
      g_free(id);
      return (SWResult){NULL, EINVAL};
    }
    g_set_prgname(id);
    g_free(id);
  }
  const char *requested = getenv("SCOOP_WINDOW_BACKEND");
  const char *backend = getenv("WAYLAND_DISPLAY") ? "wayland" : "x11";
  if (requested && *requested) {
    if (strcmp(requested, "x11") && strcmp(requested, "wayland"))
      return (SWResult){NULL, EINVAL};
    backend = requested;
  }
  gdk_set_allowed_backends(backend);
  if (!gtk_init_check())
    return (SWResult){NULL, -2};
  adw_init();
  GdkDisplay *display = gdk_display_get_default();
  int kind = GDK_IS_WAYLAND_DISPLAY(display) ? SW_WAYLAND : SW_X11;
  if ((kind == SW_WAYLAND) != !strcmp(backend, "wayland"))
    return (SWResult){NULL, -2};
  atomic_store(&selected_backend, kind);
  SWApplication *app = calloc(1, sizeof(*app));
  if (!app)
    return (SWResult){NULL, ENOMEM};
  app->wake_fd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
  if (app->wake_fd < 0) {
    int error = errno;
    free(app);
    return (SWResult){NULL, error};
  }
  GMainContext *main_context = g_main_context_default();
  if (!g_main_context_acquire(main_context)) {
    close(app->wake_fd);
    free(app);
    return (SWResult){NULL, EBUSY};
  }
  app->kind = kind;
  if (length > 0)
    app->application_id = g_strndup((const char *)identifier, (gsize)length);
  app->callback = callback;
  app->context = context;
  app->deadline = -1;
  app->main_context = g_main_context_ref(main_context);
  app->display = g_object_ref(display);
  app->monitors = gdk_display_get_monitors(display);
  app->display_closed =
      g_signal_connect(display, "closed", G_CALLBACK(display_closed), app);
  app->monitors_changed = g_signal_connect(
      app->monitors, "items-changed", G_CALLBACK(sw_monitors_changed), app);
  app->theme_changed =
      g_signal_connect(adw_style_manager_get_default(), "notify::dark",
                       G_CALLBACK(sw_theme_changed), app);
  sw_application = app;
  return (SWResult){app, 0};
}
int32_t scoop_window_application_run(SWApplication *app) {
  while (!app->stopped) {
    if (app->error && !app->error_reported) {
      app->error_reported = true;
      return app->error;
    }
    sw_watch_retire(app);
    if (app->stopped)
      break;
    int priority, gtk_timeout;
    gboolean ready = g_main_context_prepare(app->main_context, &priority);
    int capacity = g_main_context_query(app->main_context, priority,
                                        &gtk_timeout, NULL, 0);
    GPollFD *gtk_fds =
        calloc(capacity ? (size_t)capacity : 1, sizeof(*gtk_fds));
    if (!gtk_fds)
      return ENOMEM;
    int gtk_count = g_main_context_query(app->main_context, priority,
                                         &gtk_timeout, gtk_fds, capacity);
    if (gtk_count > capacity) {
      free(gtk_fds);
      continue;
    }
    size_t count = sw_watch_count(app);
    struct pollfd *fds = calloc(1 + count + gtk_count, sizeof(*fds));
    SWWatch **watches = calloc(count ? count : 1, sizeof(*watches));
    if (!fds || !watches) {
      free(gtk_fds);
      free(fds);
      free(watches);
      return ENOMEM;
    }
    fds[0] = (struct pollfd){app->wake_fd, POLLIN, 0};
    sw_watch_poll(app, fds + 1, watches);
    for (int i = 0; i < gtk_count; ++i)
      fds[1 + count + i] = (struct pollfd){gtk_fds[i].fd, gtk_fds[i].events, 0};
    int timeout = ready ? 0 : gtk_timeout;
    if (app->deadline >= 0) {
      int deadline =
          (int)fmin(INT_MAX, ceil(fmax(0, app->deadline - sw_now()) * 1000));
      if (timeout < 0 || deadline < timeout)
        timeout = deadline;
    }
    int result = poll(fds, 1 + count + gtk_count, timeout);
    if (result < 0) {
      int error = errno;
      free(gtk_fds);
      free(fds);
      free(watches);
      if (error == EINTR)
        continue;
      return error;
    }
    bool wake = (fds[0].revents & POLLIN) != 0;
    if (wake) {
      uint64_t value;
      while (read(app->wake_fd, &value, sizeof(value)) < 0 && errno == EINTR) {
      }
    }
    for (int i = 0; i < gtk_count; ++i)
      gtk_fds[i].revents = fds[1 + count + i].revents;
    if (g_main_context_check(app->main_context, priority, gtk_fds, gtk_count))
      g_main_context_dispatch(app->main_context);
    free(gtk_fds);
    gdk_gl_context_clear_current();
    // Managed handlers run after GTK has left its signal and frame-clock stack.
    sw_watch_dispatch(app, fds + 1, count, watches);
    free(watches);
    free(fds);
    bool expired = app->deadline >= 0 && sw_now() >= app->deadline;
    if (expired)
      app->deadline = -1;
    if (wake || expired || app->head)
      app->callback(app->context, 0, expired ? 1 : 0);
  }
  return 0;
}
void scoop_window_application_wake(SWApplication *app) { sw_wake(app); }
void scoop_window_application_deadline(SWApplication *app, double seconds) {
  app->deadline = seconds < 0 ? -1 : sw_now() + seconds;
  sw_wake(app);
}
void scoop_window_application_stop(SWApplication *app) {
  app->stopped = true;
  sw_wake(app);
}
void scoop_window_application_destroy(SWApplication *app) {
  sw_application = NULL;
  finish(app);
}
