#include "delivery.h"

SWResult scoop_window_input_copy(SWEvent *event) {
  if (!event->native_input)
    return (SWResult){NULL, 0};
  SWNativeInput *input = malloc(sizeof(*input));
  if (!input)
    return (SWResult){NULL, ERROR_NOT_ENOUGH_MEMORY};
  int32_t code = event->header.kind == SWKeyboard
                     ? (int32_t)event->keyboard.physical_code
                 : event->header.kind == SWContact ? 0
                                                   : event->pointer.button;
  *input = (SWNativeInput){
      event->header.window, event->header.kind, code, event->input_position,
      event->header.kind == SWContact ? event->contact.id : 0};
  return (SWResult){input, 0};
}
void scoop_window_input_release(void *input) { free(input); }
