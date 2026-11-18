#ifndef SCOOP_WINDOW_DRAG_DROP_H
#define SCOOP_WINDOW_DRAG_DROP_H

#include <stdint.h>

typedef int32_t (*SWDragCallback)(void *, int32_t, int32_t);
typedef int32_t (*SWDropCallback)(void *, int32_t, int32_t, int32_t, int32_t,
                                  double, double, int64_t, int32_t, void *);
typedef struct {
  const uint8_t *bytes;
  int64_t length;
} SWDropBytes;

static inline int32_t sw_drop_choice(int32_t choice, int32_t formats,
                                     int32_t effects) {
  int32_t format = choice / 256, effect = choice & 255;
  return format >= 1 && format <= 3 && (formats & (1 << (format - 1))) &&
                 (effect == 1 || effect == 2 || effect == 4) &&
                 (effects & effect)
             ? choice
             : 0;
}

#endif
