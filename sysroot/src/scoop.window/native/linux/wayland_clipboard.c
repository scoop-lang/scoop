#include "wayland.h"
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>

static void offer_mime(void *data, struct wl_data_offer *offer,
                       const char *mime) {
  SLOffer *value = data;
  if (!strcmp(mime, "text/plain;charset=utf-8"))
    value->mime = "text/plain;charset=utf-8";
  else if (!value->mime && !strcmp(mime, "text/plain"))
    value->mime = "text/plain";
}
static void offer_actions(void *data, struct wl_data_offer *offer,
                          uint32_t actions) {}
static void offer_action(void *data, struct wl_data_offer *offer,
                         uint32_t action) {}
static const struct wl_data_offer_listener offer_listener = {
    .offer = offer_mime,
    .source_actions = offer_actions,
    .action = offer_action};

static void data_offer(void *data, struct wl_data_device *device,
                       struct wl_data_offer *offer) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  SLOffer *value = calloc(1, sizeof(*value));
  if (!value) {
    wl_data_offer_destroy(offer);
    sw_fail(app, ENOMEM);
    return;
  }
  value->offer = offer;
  value->next = w->offers;
  w->offers = value;
  wl_data_offer_add_listener(offer, &offer_listener, value);
}
static void selection(void *data, struct wl_data_device *device,
                      struct wl_data_offer *offer) {
  SLApplication *w = ((SWApplication *)data)->backend;
  w->selection = NULL;
  SLOffer **link = &w->offers;
  while (*link) {
    SLOffer *value = *link;
    if (value->offer == offer) {
      w->selection = value;
      link = &value->next;
    } else {
      *link = value->next;
      wl_data_offer_destroy(value->offer);
      free(value);
    }
  }
}
static void drag_enter(void *data, struct wl_data_device *device,
                       uint32_t serial, struct wl_surface *surface,
                       wl_fixed_t x, wl_fixed_t y,
                       struct wl_data_offer *offer) {
  // File drag-and-drop is outside the current window API; reject its offer.
  if (offer)
    wl_data_offer_accept(offer, serial, NULL);
}
static void drag_leave(void *data, struct wl_data_device *device) {}
static void drag_motion(void *data, struct wl_data_device *device,
                        uint32_t time, wl_fixed_t x, wl_fixed_t y) {}
static void drag_drop(void *data, struct wl_data_device *device) {}
static const struct wl_data_device_listener data_listener = {
    .data_offer = data_offer,
    .enter = drag_enter,
    .leave = drag_leave,
    .motion = drag_motion,
    .drop = drag_drop,
    .selection = selection};

static void source_target(void *data, struct wl_data_source *source,
                          const char *mime) {}
static void source_send(void *data, struct wl_data_source *source,
                        const char *mime, int32_t fd) {
  SLSource *value = data;
  if (value->cancelled || (strcmp(mime, "text/plain;charset=utf-8") &&
                           strcmp(mime, "text/plain"))) {
    close(fd);
    return;
  }
  SLApplication *w = value->application->backend;
  SLTransfer *transfer = calloc(1, sizeof(*transfer));
  uint8_t *text = malloc(value->length ? value->length : 1);
  int flags = fcntl(fd, F_GETFL);
  if (!transfer || !text || flags < 0 ||
      fcntl(fd, F_SETFL, flags | O_NONBLOCK) < 0) {
    free(transfer);
    free(text);
    close(fd);
    return;
  }
  fcntl(fd, F_SETFD, FD_CLOEXEC);
  memcpy(text, value->text, value->length);
  *transfer = (SLTransfer){
      .fd = fd, .text = text, .length = value->length, .next = w->transfers};
  w->transfers = transfer;
  sw_wake(value->application);
}
static void source_cancelled(void *data, struct wl_data_source *source) {
  SLSource *value = data;
  SLApplication *w = value->application->backend;
  value->cancelled = true;
  if (w->current_source == value)
    w->current_source = NULL;
  SLSource **link = &w->sources;
  while (*link && *link != value)
    link = &(*link)->next;
  if (*link)
    *link = value->next;
  wl_data_source_destroy(source);
  free(value->text);
  free(value);
}
static void source_drop(void *data, struct wl_data_source *source) {}
static void source_finished(void *data, struct wl_data_source *source) {}
static void source_action(void *data, struct wl_data_source *source,
                          uint32_t action) {}
static const struct wl_data_source_listener source_listener = {
    .target = source_target,
    .send = source_send,
    .cancelled = source_cancelled,
    .dnd_drop_performed = source_drop,
    .dnd_finished = source_finished,
    .action = source_action};

void sl_setup_clipboard(SWApplication *app) {
  SLApplication *w = app->backend;
  if (!w->data_device && w->data_manager && w->seat) {
    w->data_device =
        wl_data_device_manager_get_data_device(w->data_manager, w->seat);
    wl_data_device_add_listener(w->data_device, &data_listener, app);
  }
}

static ssize_t write_pipe(int fd, const uint8_t *text, size_t size) {
  sigset_t blocked, previous, pending;
  sigemptyset(&blocked);
  sigaddset(&blocked, SIGPIPE);
  int status = pthread_sigmask(SIG_BLOCK, &blocked, &previous);
  if (status) {
    errno = status;
    return -1;
  }
  sigpending(&pending);
  bool had_signal = sigismember(&pending, SIGPIPE);
  ssize_t result = write(fd, text, size);
  int error = errno;
  if (result < 0 && error == EPIPE && !had_signal) {
    struct timespec zero = {0};
    while (sigtimedwait(&blocked, NULL, &zero) < 0 && errno == EINTR) {
    }
  }
  pthread_sigmask(SIG_SETMASK, &previous, NULL);
  errno = error;
  return result;
}

void sl_clipboard_flush(SWApplication *app) {
  SLApplication *w = app->backend;
  SLTransfer **link = &w->transfers;
  while (*link) {
    SLTransfer *transfer = *link;
    bool finished = transfer->offset == transfer->length;
    while (!finished) {
      ssize_t count =
          write_pipe(transfer->fd, transfer->text + transfer->offset,
                     transfer->length - transfer->offset);
      if (count < 0 && errno == EINTR)
        continue;
      if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK))
        break;
      if (count <= 0) {
        finished = true;
        break;
      }
      transfer->offset += count;
      finished = transfer->offset == transfer->length;
    }
    if (finished) {
      *link = transfer->next;
      close(transfer->fd);
      free(transfer->text);
      free(transfer);
    } else
      link = &transfer->next;
  }
}

size_t sw_wayland_transfer_count(SWApplication *app) {
  size_t count = 0;
  for (SLTransfer *transfer = ((SLApplication *)app->backend)->transfers;
       transfer; transfer = transfer->next)
    ++count;
  return count;
}
void sw_wayland_transfer_poll(SWApplication *app, struct pollfd *fds) {
  size_t index = 0;
  for (SLTransfer *transfer = ((SLApplication *)app->backend)->transfers;
       transfer; transfer = transfer->next)
    fds[index++] = (struct pollfd){transfer->fd, POLLOUT, 0};
}
void sw_wayland_transfer_flush(SWApplication *app) { sl_clipboard_flush(app); }

SWClipboardText sw_wayland_clipboard_read(SWApplication *app) {
  SLApplication *w = app->backend;
  int error = sw_wayland_dispatch(app);
  if (error)
    return (SWClipboardText){.error = error};
  if (w->current_source && !w->current_source->cancelled) {
    SLSource *source = w->current_source;
    uint8_t *copy = malloc(source->length + 1);
    if (!copy)
      return (SWClipboardText){.error = ENOMEM};
    memcpy(copy, source->text, source->length);
    copy[source->length] = 0;
    return (SWClipboardText){copy, source->length, 0};
  }
  if (!w->selection || !w->selection->mime)
    return (SWClipboardText){0};
  int pipe_fds[2];
  if (pipe2(pipe_fds, O_CLOEXEC))
    return (SWClipboardText){.error = errno};
  if (fcntl(pipe_fds[0], F_SETFL, O_NONBLOCK) < 0) {
    int status = errno;
    close(pipe_fds[0]);
    close(pipe_fds[1]);
    return (SWClipboardText){.error = status};
  }
  wl_data_offer_receive(w->selection->offer, w->selection->mime, pipe_fds[1]);
  close(pipe_fds[1]);
  SWClipboardText result = {0};
  double deadline = sw_now() + 5;
  for (;;) {
    int error = sw_wayland_flush(app);
    if (!error)
      error = sw_wayland_dispatch(app);
    if (error) {
      result.error = error;
      break;
    }
    uint8_t buffer[8192];
    ssize_t count = read(pipe_fds[0], buffer, sizeof(buffer));
    if (count == 0)
      break;
    if (count > 0) {
      if (result.length > INT64_MAX - count - 1) {
        result.error = EOVERFLOW;
        break;
      }
      uint8_t *next = realloc(result.bytes, result.length + count + 1);
      if (!next) {
        result.error = ENOMEM;
        break;
      }
      memcpy(next + result.length, buffer, count);
      result.length += count;
      result.bytes = next;
      continue;
    }
    if (errno == EINTR)
      continue;
    if (errno != EAGAIN && errno != EWOULDBLOCK) {
      result.error = errno;
      break;
    }
    double remaining = deadline - sw_now();
    if (remaining <= 0) {
      result.error = ETIMEDOUT;
      break;
    }
    size_t transfers = sw_wayland_transfer_count(app);
    struct pollfd *fds = calloc(transfers + 2, sizeof(*fds));
    if (!fds) {
      result.error = ENOMEM;
      break;
    }
    fds[0] = (struct pollfd){pipe_fds[0], POLLIN, 0};
    fds[1] =
        (struct pollfd){app->display_fd,
                        POLLIN | (app->display_write_pending ? POLLOUT : 0), 0};
    sw_wayland_transfer_poll(app, fds + 2);
    int status = poll(fds, transfers + 2, (int)ceil(remaining * 1000));
    free(fds);
    if (status < 0 && errno != EINTR) {
      result.error = errno;
      break;
    }
  }
  close(pipe_fds[0]);
  if (!result.bytes && !result.error)
    result.bytes = malloc(1);
  if (result.bytes)
    result.bytes[result.length] = 0;
  else if (!result.error)
    result.error = ENOMEM;
  if (!result.error && !sw_utf8_valid(result.bytes, result.length))
    result.error = EILSEQ;
  return result;
}

int64_t sw_wayland_clipboard_write(SWApplication *app, const uint8_t *text,
                                   size_t length) {
  SLApplication *w = app->backend;
  if (!w->data_device || !w->serial)
    return -2;
  SLSource *source = calloc(1, sizeof(*source));
  uint8_t *copy = malloc(length ? length : 1);
  if (!source || !copy) {
    free(source);
    free(copy);
    return ENOMEM;
  }
  memcpy(copy, text, length);
  source->application = app;
  source->text = copy;
  source->length = length;
  source->source = wl_data_device_manager_create_data_source(w->data_manager);
  if (!source->source) {
    free(copy);
    free(source);
    return ENOMEM;
  }
  source->next = w->sources;
  w->sources = source;
  wl_data_source_add_listener(source->source, &source_listener, source);
  wl_data_source_offer(source->source, "text/plain;charset=utf-8");
  wl_data_source_offer(source->source, "text/plain");
  wl_data_device_set_selection(w->data_device, source->source, w->serial);
  w->current_source = source;
  return sw_wayland_flush(app);
}

void sl_finish_clipboard(SWApplication *app) {
  SLApplication *w = app->backend;
  while (w->transfers) {
    SLTransfer *transfer = w->transfers;
    w->transfers = transfer->next;
    close(transfer->fd);
    free(transfer->text);
    free(transfer);
  }
  while (w->sources) {
    SLSource *source = w->sources;
    w->sources = source->next;
    wl_data_source_destroy(source->source);
    free(source->text);
    free(source);
  }
  while (w->offers) {
    SLOffer *offer = w->offers;
    w->offers = offer->next;
    wl_data_offer_destroy(offer->offer);
    free(offer);
  }
  if (w->data_device)
    wl_data_device_release(w->data_device);
  if (w->data_manager)
    wl_data_device_manager_destroy(w->data_manager);
}
