#ifndef SCOOP_WINDOW_INPUT_H
#define SCOOP_WINDOW_INPUT_H

#include <stdbool.h>
#include <stdint.h>

enum { SWKeyboard = 10, SWModifiers = 11, SWPointer = 12, SWGesture = 20 };

typedef enum {
  SWGestureMagnify,
  SWGestureRotate,
  SWGestureSwipe
} SWGestureKind;

typedef enum {
  SWGestureNone,
  SWGestureBegan,
  SWGestureChanged,
  SWGestureEnded,
  SWGestureCancelled
} SWGesturePhase;

typedef struct {
  int32_t kind, phase;
  int64_t modifiers;
  double x, y, physical_x, physical_y, scale, delta_x, delta_y;
} SWGestureData;

typedef struct {
  bool active;
  SWGestureData latest;
} SWGestureSequence;

typedef enum {
  SWPointerEntered,
  SWPointerLeft,
  SWPointerMoved,
  SWPointerPressed,
  SWPointerReleased,
  SWPointerLineWheel,
  SWPointerPixelWheel,
  SWPointerCancelled
} SWPointerKind;

typedef enum {
  SWScrollNone,
  SWScrollMayBegin,
  SWScrollBegan,
  SWScrollStationary,
  SWScrollChanged,
  SWScrollEnded,
  SWScrollCancelled
} SWScrollPhase;

typedef enum {
  SWPointerFocusLost,
  SWPointerCaptureLost,
  SWPointerSystemOperation,
  SWPointerModal
} SWPointerCancelReason;

typedef struct {
  int64_t physical_code, logical_code, modifiers;
  int32_t pressed, repeat, synthetic, dead;
  const uint8_t *text;
  int64_t text_length;
} SWKeyboardData;

typedef struct {
  int32_t kind, button, click_count, phase, momentum_phase, cancel_reason;
  int64_t modifiers;
  double x, y, physical_x, physical_y, scale, delta_x, delta_y;
} SWPointerData;

/* Native surface-pixel deltas are logical; the Window API uses physical pixels.
 */
static inline void sw_pointer_scroll(SWPointerData *data, bool pixels,
                                     double dx, double dy, SWScrollPhase phase,
                                     SWScrollPhase momentum) {
  data->kind = pixels ? SWPointerPixelWheel : SWPointerLineWheel;
  data->delta_x = dx * (pixels ? data->scale : 1.0);
  data->delta_y = dy * (pixels ? data->scale : 1.0);
  data->phase = phase;
  data->momentum_phase = momentum;
}

#endif
