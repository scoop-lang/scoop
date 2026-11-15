#ifndef SCOOP_WINDOW_PREFERENCES_H
#define SCOOP_WINDOW_PREFERENCES_H

#include <stdint.h>

// Negative booleans and intervals, or zero sizes, represent unavailable facts.
typedef struct {
  int32_t auto_hide_scrollbars, reduce_motion, high_contrast,
      reduce_transparency;
  double text_scale_factor, caret_blink_seconds;
  int32_t caret_width_pixels;
  double caret_aspect_ratio, double_click_seconds;
} SWInteractionPreferences;

static inline SWInteractionPreferences sw_unknown_preferences(void) {
  SWInteractionPreferences value = {-1, -1, -1, -1, 0, -1, 0, 0, 0};
  return value;
}

#endif
