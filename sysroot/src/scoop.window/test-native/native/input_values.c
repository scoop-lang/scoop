#include "../../native/input.h"

SWContactData scoop_window_test_contact_data(int32_t kind, int32_t phase,
                                             int32_t available) {
  return (SWContactData){.id = INT64_C(4294967311),
                         .device = INT64_C(9000000007),
                         .kind = kind,
                         .phase = phase,
                         .cancel_reason = SWPointerSystemOperation,
                         .has_device = available,
                         .has_pressure = available,
                         .x = 12.5,
                         .y = 9.25,
                         .physical_x = 18.75,
                         .physical_y = 13.875,
                         .scale = 1.5,
                         .pressure = 0.75};
}

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
