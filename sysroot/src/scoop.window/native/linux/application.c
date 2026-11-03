#include "window.h"
#include <locale.h>
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

static void finish(SWApplication *app) {
  if (app->kind == SW_X11)
    sw_x11_finish(app);
  else
    sw_wayland_finish(app);
  xkb_state_unref(app->key_state);
  xkb_keymap_unref(app->keymap);
  xkb_compose_table_unref(app->compose_table);
  xkb_context_unref(app->xkb);
  if (app->wake_fd >= 0)
    close(app->wake_fd);
  while (app->head) {
    SWEvent *event = app->head;
    app->head = event->next;
    sw_free_event(event);
  }
  free(app);
}

SWResult scoop_window_application_create(SWCallback callback, void *context,
                                         int32_t policy) {
  const char *backend = getenv("SCOOP_WINDOW_BACKEND");
  int kind = getenv("WAYLAND_DISPLAY") ? SW_WAYLAND : SW_X11;
  if (backend && *backend) {
    if (!strcmp(backend, "x11"))
      kind = SW_X11;
    else if (!strcmp(backend, "wayland"))
      kind = SW_WAYLAND;
    else
      return (SWResult){NULL, EINVAL};
  }
  atomic_store(&selected_backend, kind);
  SWApplication *app = calloc(1, sizeof(*app));
  if (!app)
    return (SWResult){NULL, ENOMEM};
  app->kind = kind;
  app->wake_fd = -1;
  app->display_fd = -1;
  app->decor_fd = -1;
  app->callback = callback;
  app->context = context;
  app->deadline = -1;
  app->xkb = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
  app->wake_fd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
  if (!app->xkb || app->wake_fd < 0) {
    int error = errno ? errno : ENOMEM;
    finish(app);
    return (SWResult){NULL, error};
  }
  const char *locale = getenv("LC_ALL");
  if (!locale || !*locale)
    locale = getenv("LC_CTYPE");
  if (!locale || !*locale)
    locale = getenv("LANG");
  app->compose_table = xkb_compose_table_new_from_locale(
      app->xkb, locale && *locale ? locale : "C.UTF-8",
      XKB_COMPOSE_COMPILE_NO_FLAGS);
  sw_application = app;
  int error =
      kind == SW_X11 ? sw_x11_initialize(app) : sw_wayland_initialize(app);
  if (error) {
    sw_application = NULL;
    finish(app);
    return (SWResult){NULL, error};
  }
  sw_application = app;
  return (SWResult){app, 0};
}

int sw_dispatch_native(SWApplication *app) {
  return app->kind == SW_X11 ? sw_x11_dispatch(app) : sw_wayland_dispatch(app);
}

int sw_flush(SWApplication *app) {
  return app->kind == SW_X11 ? sw_x11_flush(app) : sw_wayland_flush(app);
}

int32_t scoop_window_application_run(SWApplication *app) {
  // After a disconnect, continue owner cleanup without touching the dead
  // server.
  while (!app->stopped) {
    if (app->error && !app->error_reported) {
      app->error_reported = true;
      return app->error;
    }
    if (!app->error) {
      int error = sw_dispatch_native(app);
      if (error) {
        app->error = error;
        app->error_reported = true;
        return error;
      }
      error = sw_flush(app);
      if (error) {
        app->error = error;
        app->error_reported = true;
        return error;
      }
    }
    sw_watch_retire(app);
    if (app->stopped)
      break;
    size_t count = sw_watch_count(app);
    size_t transfers =
        app->kind == SW_WAYLAND ? sw_wayland_transfer_count(app) : 0;
    struct pollfd *fds = calloc(count + transfers + 3, sizeof(*fds));
    SWWatch **watches = calloc(count ? count : 1, sizeof(*watches));
    if (!fds || !watches) {
      free(fds);
      free(watches);
      return ENOMEM;
    }
    fds[0] = (struct pollfd){app->wake_fd, POLLIN, 0};
    fds[1] =
        (struct pollfd){app->error ? -1 : app->display_fd,
                        POLLIN | (app->display_write_pending ? POLLOUT : 0), 0};
    fds[2] = (struct pollfd){app->error ? -1 : app->decor_fd, POLLIN, 0};
    sw_watch_poll(app, fds + 3, watches);
    if (transfers)
      sw_wayland_transfer_poll(app, fds + 3 + count);
    double deadline = app->deadline;
    if (app->kind == SW_WAYLAND && !app->error) {
      double repeat = sw_wayland_repeat_deadline(app);
      if (repeat >= 0 && (deadline < 0 || repeat < deadline))
        deadline = repeat;
    }
    double remaining = deadline < 0 ? -1 : fmax(0, deadline - sw_now());
    int timeout =
        remaining < 0 ? -1 : (int)fmin(INT_MAX, ceil(remaining * 1000));
    int result = poll(fds, count + transfers + 3, timeout);
    if (result < 0) {
      int error = errno;
      free(watches);
      free(fds);
      if (error == EINTR)
        continue;
      return error;
    }
    if ((fds[1].revents | fds[2].revents) & (POLLERR | POLLHUP | POLLNVAL)) {
      free(watches);
      free(fds);
      app->error = EPIPE;
      app->error_reported = true;
      return EPIPE;
    }
    bool wake = (fds[0].revents & POLLIN) != 0;
    if (wake) {
      uint64_t value;
      while (read(app->wake_fd, &value, sizeof(value)) < 0 && errno == EINTR) {
      }
    }
    if (!app->error && (fds[1].revents || fds[2].revents)) {
      int error = sw_dispatch_native(app);
      if (error) {
        app->error = error;
        app->error_reported = true;
        free(watches);
        free(fds);
        return error;
      }
    }
    sw_watch_dispatch(app, fds + 3, count, watches);
    if (app->kind == SW_WAYLAND)
      sw_wayland_transfer_flush(app);
    free(watches);
    free(fds);
    if (app->kind == SW_WAYLAND && !app->error)
      sw_wayland_repeat(app);
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
