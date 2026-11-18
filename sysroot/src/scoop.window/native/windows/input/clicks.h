#ifndef SCOOP_WINDOW_WINDOWS_CLICKS_H
#define SCOOP_WINDOW_WINDOWS_CLICKS_H

#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>

/* Screen-pixel history for the aggregate Win32 mouse, independent of HWND APIs.
 */
typedef struct {
  double x, y;
  uint32_t time;
  int button, count;
  bool reset;
} SWClicks;

static inline void sw_clicks_reset(SWClicks *clicks) {
  clicks->count = 0;
  clicks->reset = true;
}

static inline int sw_clicks_press(SWClicks *clicks, uint32_t time, int button,
                                  double x, double y, uint32_t interval,
                                  double width, double height,
                                  bool native_double) {
  bool repeated = clicks->count > 0 && clicks->button == button &&
                  (uint32_t)(time - clicks->time) <= interval &&
                  x >= clicks->x - width / 2.0 && x < clicks->x + width / 2.0 &&
                  y >= clicks->y - height / 2.0 && y < clicks->y + height / 2.0;
  int count = repeated
                  ? (clicks->count == INT_MAX ? INT_MAX : clicks->count + 1)
              : clicks->count == 0 && !clicks->reset && native_double ? 2
                                                                      : 1;
  clicks->x = x;
  clicks->y = y;
  clicks->time = time;
  clicks->button = button;
  clicks->count = count;
  clicks->reset = false;
  return count;
}

static inline void sw_clicks_move(SWClicks *clicks, double x, double y,
                                  double horizontal, double vertical) {
  if (clicks->count > 0 && (fabs(x - clicks->x) > fabs(horizontal) ||
                            fabs(y - clicks->y) > fabs(vertical)))
    sw_clicks_reset(clicks);
}

#endif
