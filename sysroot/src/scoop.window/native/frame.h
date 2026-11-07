#ifndef SCOOP_WINDOW_FRAME_H
#define SCOOP_WINDOW_FRAME_H

#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

enum {
  SW_FRAME_CLIENT = 1,
  SW_FRAME_EXTENDED = 2,
  SW_FRAME_SYSTEM_RESIZE = 4,
  SW_FRAME_MOVE = 8,
  SW_FRAME_RESIZE = 16,
  SW_FRAME_MENU = 32,
  SW_FRAME_REGION = 64,
  SW_FRAME_INPUT = 128,
  SW_FRAME_IGNORE = 256,
  SW_FRAME_SHADOW = 512,
  SW_FRAME_RESIZE_HIT = 1024,
  SW_FRAME_BUTTON_HIT = 2048
};
typedef struct {
  double x, y, width, height;
} SWFrameRect;
typedef struct {
  int32_t mode, ready;
  SWFrameRect layout;
  double border;
} SWFrameSnapshot;
typedef struct {
  SWFrameRect rect;
  int32_t role;
} SWHitRect;

static inline SWFrameRect sw_frame_unknown(void) {
  return (SWFrameRect){NAN, NAN, NAN, NAN};
}
static inline int sw_frame_contains(SWFrameRect rect, double x, double y) {
  return x >= rect.x && y >= rect.y && x < rect.x + rect.width &&
         y < rect.y + rect.height;
}
static inline int sw_frame_hit(const SWHitRect *rects, size_t count, double x,
                               double y) {
  for (size_t index = 0; index < count; ++index)
    if (sw_frame_contains(rects[index].rect, x, y))
      return rects[index].role;
  return 0;
}
static inline int sw_frame_rect_valid(SWFrameRect rect) {
  return isfinite(rect.x) && isfinite(rect.y) && isfinite(rect.width) &&
         isfinite(rect.height) && rect.width >= 0 && rect.height >= 0 &&
         isfinite(rect.x + rect.width) && isfinite(rect.y + rect.height);
}

#endif
