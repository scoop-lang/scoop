#define _POSIX_C_SOURCE 200809L
#include "http1.h"
#include <assert.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

uint64_t scoop_http1_new(int32_t, http1_config);
void scoop_http1_free(uint64_t);
http_step scoop_http1_receive(uint64_t, const uint8_t *, int64_t);
uint64_t scoop_http1_take_events(uint64_t);

static uint64_t calls_malloc, calls_calloc, calls_realloc, calls_free,
    bytes_alloc;
static int counting;
#ifdef ALLOC_COUNT
void *__real_malloc(size_t);
void *__real_calloc(size_t, size_t);
void *__real_realloc(void *, size_t);
void __real_free(void *);
void *__wrap_malloc(size_t n) {
  if (counting) {
    calls_malloc++;
    bytes_alloc += n;
  }
  return __real_malloc(n);
}
void *__wrap_calloc(size_t n, size_t s) {
  if (counting) {
    calls_calloc++;
    bytes_alloc += n * s;
  }
  return __real_calloc(n, s);
}
void *__wrap_realloc(void *p, size_t n) {
  if (counting) {
    calls_realloc++;
    bytes_alloc += n;
  }
  return __real_realloc(p, n);
}
void __wrap_free(void *p) {
  if (counting && p)
    calls_free++;
  __real_free(p);
}
#endif
static uint64_t now(clockid_t which) {
  struct timespec t;
  assert(clock_gettime(which, &t) == 0);
  return (uint64_t)t.tv_sec * 1000000000 + t.tv_nsec;
}
int main(int argc, char **argv) {
  assert(argc == 5);
  const int bridge = strcmp(argv[1], "bridge") == 0;
  const uint64_t repeats = strtoull(argv[2], NULL, 10);
  const int split = atoi(argv[3]);
  const int count_alloc = atoi(argv[4]);
  static const uint8_t request[] =
      "GET /small HTTP/1.1\r\nHost: 127.0.0.1:18081\r\n\r\n";
  const size_t length = sizeof(request) - 1;
  llhttp_settings_t settings;
  llhttp_settings_init(&settings);
  llhttp_t parser;
  llhttp_init(&parser, HTTP_REQUEST, &settings);
  uint64_t handle =
      scoop_http1_new(1, (http1_config){65536, 8192, 100, 8192, 0, 0});
  assert(handle);
  uint64_t checksum = 0;
  uint64_t wall = now(CLOCK_MONOTONIC);
  uint64_t cpu = now(CLOCK_PROCESS_CPUTIME_ID);
  counting = count_alloc;
  for (uint64_t i = 0; i < repeats; i++) {
    for (size_t offset = 0; offset < length;) {
      size_t count = split ? 1 : length;
      if (bridge) {
        http_step step = scoop_http1_receive(handle, request + offset, count);
        assert(step.count == (int64_t)count && step.code == 0);
        checksum += step.count;
      } else {
        assert(llhttp_execute(&parser, (const char *)request + offset, count) ==
               HPE_OK);
        checksum += count;
      }
      offset += count;
    }
    if (bridge) {
      http_event *event;
      http_events pending = {(void *)(uintptr_t)scoop_http1_take_events(handle),
                             NULL};
      unsigned events = 0;
      while ((event = http_events_pop(&pending))) {
        checksum += event->info.kind;
        if (event->info.kind == HTTP_EVENT_REQUEST) {
          assert(event->fields.size == 1 && event->bytes[1].size == 6);
          checksum += event->fields.items[0].value.size;
        }
        events++;
        http_event_free(event);
      }
      assert(events == 3);
    }
  }
  counting = 0;
  cpu = now(CLOCK_PROCESS_CPUTIME_ID) - cpu;
  wall = now(CLOCK_MONOTONIC) - wall;
  printf("{\"mode\":\"%s\",\"requests\":%" PRIu64
         ",\"request_bytes\":%zu,\"split\":%d,\"count_alloc\":%d,\"cpu_ns_per_"
         "request\":%.3f,"
         "\"wall_ns_per_request\":%.3f,\"checksum\":%" PRIu64
         ",\"malloc\":%" PRIu64 ",\"calloc\":%" PRIu64 ",\"realloc\":%" PRIu64
         ",\"free_nonnull\":%" PRIu64 ",\"requested_alloc_bytes\":%" PRIu64
         "}\n",
         argv[1], repeats, length, split, count_alloc, (double)cpu / repeats,
         (double)wall / repeats, checksum, calls_malloc, calls_calloc,
         calls_realloc, calls_free, bytes_alloc);
  scoop_http1_free(handle);
}
