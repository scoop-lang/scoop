#define WIN32_LEAN_AND_MEAN
#include <shobjidl.h>
#include <stdint.h>
#include <windows.h>

int32_t scoop_desktop_test_application(int32_t field) {
  PWSTR identifier = NULL;
  HRESULT status = GetCurrentProcessExplicitAppUserModelID(&identifier);
  int32_t matches =
      SUCCEEDED(status) && !wcscmp(identifier, L"dev.scoop.desktop.tests");
  CoTaskMemFree(identifier);
  return field == 0 ? matches : -1;
}
int32_t scoop_desktop_test_window(void *pointer, int32_t field) {
  if (field == 2) {
    HICON before = (HICON)SendMessageW(pointer, WM_GETICON, ICON_BIG, 0);
    SendMessageW(pointer, RegisterWindowMessageW(L"TaskbarButtonCreated"), 0,
                 0);
    return before &&
           before == (HICON)SendMessageW(pointer, WM_GETICON, ICON_BIG, 0);
  }
  return field == 0 ? SendMessageW(pointer, WM_GETICON, ICON_BIG, 0) != 0 : -1;
}
int32_t scoop_desktop_test_dock(void) { return -1; }
int32_t scoop_desktop_test_dock_popup(void *window) { return -1; }
