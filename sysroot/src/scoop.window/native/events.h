#ifndef SCOOP_WINDOW_EVENTS_H
#define SCOOP_WINDOW_EVENTS_H

#include "input.h"

typedef struct {
  int64_t window;
  int32_t kind;
  int64_t time_seconds, time_nanoseconds;
} SWEventHeader;

/* Window, menu, and text-transaction metadata retain their existing fields. */
typedef struct {
  int32_t flags;
  int64_t a, b, c, d;
  double x, y, z, w, scale;
} SWEventData;

typedef union {
  SWEventData window;
  SWKeyboardData keyboard;
  SWPointerData pointer;
  int64_t modifiers;
} SWEventPayload;

typedef struct {
  int32_t kind;
  int64_t anchor, cursor;
  const uint8_t *text;
  int64_t text_length;
  int32_t has_replacement;
  int64_t replacement_start, replacement_end;
} SWTextEditData;

#endif
