#include "delivery.h"

SWResult scoop_window_input_copy(SWEvent *event) {
  if (!event->native_input)
    return (SWResult){NULL, 0};
  SWNativeInput *input = malloc(sizeof(*input));
  if (!input)
    return (SWResult){NULL, ENOMEM};
  *input =
      (SWNativeInput){event->header.window, gdk_event_ref(event->native_input)};
  return (SWResult){input, 0};
}
void scoop_window_input_release(SWNativeInput *input) {
  gdk_event_unref(input->event);
  free(input);
}
