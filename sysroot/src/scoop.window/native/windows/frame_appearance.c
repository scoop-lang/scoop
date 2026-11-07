#include "window.h"

int32_t scoop_window_windows_frame_capabilities(SWWindow *window) {
  const DWORD attributes[] = {34, 35, 36, 33, 37};
  int32_t bits = 0;
  for (int index = 0; index < 5; ++index) {
    DWORD value = 0;
    if (SUCCEEDED(DwmGetWindowAttribute(window->hwnd, attributes[index], &value,
                                        sizeof(value))))
      bits |= 1 << index;
  }
  return bits;
}

int64_t scoop_window_windows_frame_attribute(SWWindow *window,
                                             int32_t attribute,
                                             uint32_t value) {
  DWORD previous = 0;
  HRESULT status = DwmGetWindowAttribute(window->hwnd, attribute, &previous,
                                         sizeof(previous));
  if (FAILED(status))
    return status == E_INVALIDARG ? -1 : (int64_t)(uint32_t)status;
  status =
      DwmSetWindowAttribute(window->hwnd, attribute, &value, sizeof(value));
  return SUCCEEDED(status)        ? 0
         : status == E_INVALIDARG ? -1
                                  : (int64_t)(uint32_t)status;
}
