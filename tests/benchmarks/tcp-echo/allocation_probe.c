#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "scoop_rt.h"

enum { PHASES = 14, DEPTH = 16, WARMUP = 1000 };
static const char *const names[PHASES] = {
    "require_job",         "acquire_direction",  "borrow_buffer",
    "prepare_slot",        "bind_cancellation",  "submit_transfer",
    "begin_operation",     "complete_operation", "release_borrow",
    "resume_continuation", "pump_other",         "read_call_other",
    "write_call_other",    "unattributed"};

/* One coroutine on one runtime worker is the only measured mutator. The caller
 * prints totals after that worker has joined. These probes measure allocations,
 * never CPU or latency: the public metrics API also reads resident memory. */
static uint64_t completed;
static uint64_t measured;
static uint64_t totals[PHASES];
static uint64_t calls[PHASES];
static uint64_t total_bytes;
static _Thread_local int active;
static _Thread_local unsigned depth;
static _Thread_local unsigned stack[DEPTH];
static _Thread_local uint64_t bytes[PHASES];
static _Thread_local uint64_t entries[PHASES];
static _Thread_local uint64_t previous;
static _Thread_local uint64_t initial;

static uint64_t allocated(void) {
  ScoopGcMetrics metrics;
  scoop_rt_gc_debug_metrics(&metrics);
  return metrics.allocated_bytes;
}

static void account(void) {
  uint64_t current = allocated();
  if (current < previous || depth == 0)
    abort();
  bytes[stack[depth - 1]] += current - previous;
  previous = current;
}

void echo_alloc_start(void) {
  if (active)
    abort();
  memset(bytes, 0, sizeof bytes);
  memset(entries, 0, sizeof entries);
  depth = 1;
  stack[0] = PHASES - 1;
  initial = previous = allocated();
  active = 1;
}

void echo_alloc_enter(int32_t phase) {
  if (!active)
    return;
  if (phase < 1 || phase > PHASES || depth == DEPTH)
    abort();
  account();
  stack[depth++] = (unsigned)phase - 1;
  entries[phase - 1]++;
}

void echo_alloc_leave(void) {
  if (!active)
    return;
  if (depth <= 1)
    abort();
  account();
  depth--;
}

void echo_alloc_end(void) {
  if (!active || depth != 1)
    abort();
  account();
  active = 0;
  completed++;
  if (completed <= WARMUP)
    return;
  measured++;
  uint64_t sum = 0;
  for (unsigned phase = 0; phase < PHASES; phase++) {
    sum += bytes[phase];
    totals[phase] += bytes[phase];
    calls[phase] += entries[phase];
  }
  if (sum != previous - initial)
    abort();
  total_bytes += sum;
}

void echo_alloc_report(void) {
  printf("ALLOC_PHASES {\"completed\":%" PRIu64 ",\"warmup\":%d,"
         "\"measured\":%" PRIu64 ",\"allocated_bytes\":%" PRIu64
         ",\"phases\":[",
         completed, WARMUP, measured, total_bytes);
  for (unsigned phase = 0; phase < PHASES; phase++) {
    printf("%s{\"name\":\"%s\",\"bytes\":%" PRIu64 ",\"calls\":%" PRIu64 "}",
           phase == 0 ? "" : ",", names[phase], totals[phase], calls[phase]);
  }
  puts("]}");
}
