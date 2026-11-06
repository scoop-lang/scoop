#include "window.h"
#include <fcntl.h>

struct SWWatch {
  SWApplication *application;
  int64_t identity;
  int fd;
  bool removing, active;
  SWWatch *next;
};

void *scoop_window_watch_create(SWApplication *app, int64_t identity, int fd) {
  if (fd < 0 || fcntl(fd, F_GETFD) < 0) {
    errno = EBADF;
    return NULL;
  }
  SWWatch *watch = calloc(1, sizeof(*watch));
  if (!watch) {
    errno = ENOMEM;
    return NULL;
  }
  watch->application = app;
  watch->identity = identity;
  watch->fd = fd;
  watch->next = app->watches;
  app->watches = watch;
  sw_wake(app);
  return watch;
}

void scoop_window_watch_remove(SWWatch *watch) {
  watch->removing = true;
  sw_wake(watch->application);
}

size_t sw_watch_count(SWApplication *app) {
  size_t count = 0;
  for (SWWatch *watch = app->watches; watch; watch = watch->next)
    if (!watch->removing)
      ++count;
  return count;
}

void sw_watch_poll(SWApplication *app, struct pollfd *fds, SWWatch **watches) {
  size_t index = 0;
  for (SWWatch *watch = app->watches; watch; watch = watch->next) {
    if (watch->removing)
      continue;
    fds[index] = (struct pollfd){watch->fd, POLLIN, 0};
    watches[index++] = watch;
  }
}

void sw_watch_dispatch(SWApplication *app, struct pollfd *fds, size_t count,
                       SWWatch **watches) {
  for (size_t index = 0; index < count; ++index) {
    SWWatch *watch = watches[index];
    if (!fds[index].revents || watch->removing)
      continue;
    watch->active = true;
    app->callback(app->context, watch->identity, 2);
    watch->active = false;
  }
  // Retire only after all pointers in this poll snapshot have been consumed.
}

void sw_watch_retire(SWApplication *app) {
  SWWatch **link = &app->watches;
  while (*link) {
    SWWatch *watch = *link;
    if (!watch->removing || watch->active) {
      link = &watch->next;
      continue;
    }
    int64_t identity = watch->identity;
    *link = watch->next;
    free(watch);
    app->callback(app->context, identity, 3);
    link = &app->watches;
  }
}
