#import "../window.h"

SWResult scoop_window_input_copy(void *pointer) {
  SWEvent *event = (__bridge SWEvent *)pointer;
  return (SWResult){event.nativeInput ? (__bridge_retained void *)event : NULL,
                    0};
}

void scoop_window_input_release(void *pointer) {
  SWEvent *event = (__bridge_transfer SWEvent *)pointer;
  (void)event;
}
