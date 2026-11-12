#include "http1.h"
#include <stdlib.h>
#include <string.h>

uint64_t scoop_http1_new(int32_t, http1_config);
void scoop_http1_free(uint64_t);
http_step scoop_http1_receive(uint64_t, const uint8_t *, int64_t);
uint64_t scoop_http1_take_events(uint64_t);

typedef struct {
  uint64_t decoder;
  int route;
  size_t used;
  char wire[8192];
} diagnostic;

static int route(const uint8_t *p, size_t n) {
  if (n == 6 && memcmp(p, "/small", 6) == 0)
    return 1;
  if (n == 6 && memcmp(p, "/large", 6) == 0)
    return 2;
  if (n == 10 && memcmp(p, "/__metrics", 10) == 0)
    return 3;
  return -1;
}
uint64_t diag_new(int32_t parser) {
  diagnostic *s = calloc(1, sizeof(*s));
  if (!s)
    return 0;
  if (parser) {
    s->decoder =
        scoop_http1_new(1, (http1_config){65536, 8192, 100, 8192, 0, 0});
    if (!s->decoder) {
      free(s);
      return 0;
    }
  }
  return (uint64_t)(uintptr_t)s;
}
void diag_free(uint64_t handle) {
  diagnostic *s = (void *)(uintptr_t)handle;
  if (s->decoder)
    scoop_http1_free(s->decoder);
  free(s);
}
// Diagnostic GET-only path: real parser and native events, no managed
// materialization. A completed request is returned only after its End event.
// Pipelining is rejected.
int32_t diag_feed(uint64_t handle, const uint8_t *data, int64_t count) {
  diagnostic *s = (void *)(uintptr_t)handle;
  if (count < 0)
    return -1;
  if (s->decoder) {
    http_step step = scoop_http1_receive(s->decoder, data, count);
    if (step.code || step.count != count)
      return -1;
    http_event *event;
    http_events pending = {
        (void *)(uintptr_t)scoop_http1_take_events(s->decoder), NULL};
    int result = 0;
    while ((event = http_events_pop(&pending))) {
      if (result) {
        http_event_free(event);
        http_events_clear(&pending);
        return -1;
      }
      if (event->info.kind == HTTP_EVENT_REQUEST) {
        if (s->route || event->bytes[0].size != 3 ||
            memcmp(event->bytes[0].data, "GET", 3)) {
          http_event_free(event);
          http_events_clear(&pending);
          return -1;
        }
        s->route = route(event->bytes[1].data, event->bytes[1].size);
        if (s->route < 0) {
          http_event_free(event);
          http_events_clear(&pending);
          return -1;
        }
      } else if (event->info.kind == HTTP_EVENT_END) {
        result = s->route;
        s->route = 0;
      } else if (event->info.kind != HTTP_EVENT_TRAILERS) {
        http_event_free(event);
        http_events_clear(&pending);
        return -1;
      }
      http_event_free(event);
    }
    return result;
  }
  // Delimiter-only lower bound for controlled benchmark traffic, not an HTTP
  // implementation.
  if ((uint64_t)count > sizeof(s->wire) - s->used)
    return -1;
  memcpy(s->wire + s->used, data, (size_t)count);
  s->used += (size_t)count;
  if (s->used < 4 || memcmp(s->wire + s->used - 4, "\r\n\r\n", 4))
    return 0;
  if (s->used < 16 || memcmp(s->wire, "GET ", 4))
    return -1;
  const char *end = memchr(s->wire + 4, ' ', s->used - 4);
  if (!end)
    return -1;
  int result = route((uint8_t *)s->wire + 4, (size_t)(end - s->wire - 4));
  s->used = 0;
  return result;
}
