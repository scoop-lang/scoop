#include "../software_keyboard.h"
#import "window.h"

int64_t scoop_window_text_purpose(void *pointer, int64_t connection,
                                  int32_t purpose) {
  SWWindow *window = (__bridge SWWindow *)pointer;
  if (!window.view.imeAllowed || window.view.textConnection != connection)
    return -4;
  (void)purpose;
  return -1;
}
int64_t scoop_window_keyboard_request(void *pointer, int64_t connection,
                                      int32_t visible, void *input) {
  SWWindow *window = (__bridge SWWindow *)pointer;
  (void)visible;
  (void)input;
  if (window.destroying || !window.view.imeAllowed ||
      window.view.textConnection != connection)
    return 1;
  return -1;
}
SWSoftwareKeyboard scoop_window_keyboard_state(void *pointer) {
  (void)pointer;
  return (SWSoftwareKeyboard){0};
}
