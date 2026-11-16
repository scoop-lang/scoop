#include "../../native/input.h"

SWPointerData scoop_window_test_scroll_data(int32_t pixels, double scale,
                                            int32_t phase_index) {
  static const SWScrollPhase phases[] = {
      SWScrollNone,    SWScrollMayBegin, SWScrollBegan,    SWScrollStationary,
      SWScrollChanged, SWScrollEnded,    SWScrollCancelled};
  SWPointerData data = {.x = 12.5,
                        .y = 9.25,
                        .physical_x = 12.5 * scale,
                        .physical_y = 9.25 * scale,
                        .scale = scale};
  sw_pointer_scroll(&data, pixels != 0, -4, 2, phases[phase_index],
                    phases[6 - phase_index]);
  return data;
}
