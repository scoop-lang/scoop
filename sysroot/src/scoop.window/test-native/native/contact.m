#import "../../native/darwin/window.h"

// AppKit exposes no direct screen contacts. Feed controlled payloads through
// the production queue and decoder without presenting them as device events.
void scoop_window_test_contact(void *pointer, int64_t identity, int32_t kind,
                               int32_t phase, double x, double y) {
  SWView *view = (__bridge SWView *)pointer;
  double scale = view.window.backingScaleFactor;
  SWEvent *event = sw_event(view.owner, SWContact);
  event.contact = (SWContactData){.id = identity,
                                  .device = 4294967311LL,
                                  .kind = kind,
                                  .phase = phase,
                                  .cancel_reason = SWPointerCaptureLost,
                                  .has_device = 1,
                                  .has_pressure = 1,
                                  .x = x,
                                  .y = y,
                                  .physical_x = x * scale,
                                  .physical_y = y * scale,
                                  .scale = scale,
                                  .pressure = 0.5};
  [view.owner.application enqueue:event];
}
