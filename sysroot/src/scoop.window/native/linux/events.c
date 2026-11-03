#include "window.h"

void sw_stamp_input(SWEventData *event) {
  struct timespec now;
  clock_gettime(CLOCK_MONOTONIC, &now);
  event->time_seconds = now.tv_sec;
  event->time_nanoseconds = now.tv_nsec;
}

SWEvent *sw_event(SWWindow *window, int kind) {
  SWEvent *event = calloc(1, sizeof(*event));
  if (!event) {
    sw_fail(window->application, ENOMEM);
    return NULL;
  }
  event->data.window = window->identity;
  event->data.kind = kind;
  sw_stamp_input(&event->data);
  return event;
}

void sw_enqueue(SWApplication *app, SWEvent *event) {
  if (!event)
    return;
  if (app->tail)
    app->tail->next = event;
  else
    app->head = event;
  app->tail = event;
  sw_wake(app);
}

void sw_free_event(SWEvent *event) {
  if (!event)
    return;
  free((void *)event->data.text);
  for (size_t index = 0; index < event->edit_count; ++index)
    free((void *)event->edits[index].text);
  free(event->edits);
  free(event);
}

void sw_emit(SWWindow *window, int kind) {
  SWEvent *event = sw_event(window, kind);
  if (!event)
    return;
  SWSnapshot value = window->snapshot;
  if (kind == 2) {
    event->data.x = value.width;
    event->data.y = value.height;
    event->data.z = value.pixel_width;
    event->data.w = value.pixel_height;
    event->data.scale = value.scale;
  } else if (kind == 3) {
    event->data.x = value.x;
    event->data.y = value.y;
  } else if (kind == 4)
    event->data.flags = value.focused;
  else if (kind == 5)
    event->data.flags = value.occluded;
  else if (kind == 6)
    event->data.flags = value.dark;
  sw_enqueue(window->application, event);
}

void *scoop_window_next_event(SWApplication *app) {
  SWEvent *event = app->head;
  if (event) {
    app->head = event->next;
    event->next = NULL;
    if (!app->head)
      app->tail = NULL;
  }
  return event;
}

SWEventData scoop_window_event_data(SWEvent *event) { return event->data; }
void scoop_window_event_release(SWEvent *event) { sw_free_event(event); }
int64_t scoop_window_text_count(SWEvent *event) { return event->edit_count; }
SWTextEditData scoop_window_text_edit(SWEvent *event, int64_t index) {
  return event->edits[index];
}

void sw_set_geometry(SWWindow *window, double width, double height,
                     double scale) {
  double pw = ceil(width * scale), ph = ceil(height * scale);
  if (!isfinite(pw) || !isfinite(ph) || pw > INT_MAX || ph > INT_MAX) {
    sw_fail(window->application, EOVERFLOW);
    return;
  }
  SWSnapshot *value = &window->snapshot;
  bool changed = !window->configured || value->width != width ||
                 value->height != height || value->scale != scale ||
                 value->pixel_width != pw || value->pixel_height != ph;
  value->width = width;
  value->height = height;
  value->pixel_width = pw;
  value->pixel_height = ph;
  value->scale = scale;
  window->configured = true;
  if (changed) {
    window->caret_known = false;
    sw_emit(window, 2);
  }
}
