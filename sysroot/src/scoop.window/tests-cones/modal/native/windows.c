#define WIN32_LEAN_AND_MEAN
#include <stdint.h>
#include <windows.h>

int32_t scoop_test_modal_query(HWND window, HWND parent, int32_t field,
                               int32_t value) {
  switch (field) {
  case 0:
    return GetWindow(window, GW_OWNER) == parent;
  case 1:
    return IsWindowEnabled(window);
  case 2:
    return (GetWindowLongPtrW(window, GWL_EXSTYLE) & WS_EX_TOOLWINDOW) != 0;
  case 3:
    return 0;
  case 4:
    SendMessageW(window, WM_CLOSE, 0, 0);
    return 1;
  case 5:
    return 0;
  case 10:
    EnableWindow(window, value != 0);
    return 1;
  case 20:
    return 0;
  default:
    return -1;
  }
}
