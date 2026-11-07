#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <dwmapi.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>

double scoop_test_frame_query(HWND window, void *view, int32_t field, double x,
                              double y) {
  static HWND original;
  double scale = GetDpiForWindow(window) / 96.0;
  POINT point = {(LONG)round(x * scale), (LONG)round(y * scale)};
  switch (field) {
  case 0: {
    if (!(GetWindowLongPtrW(window, GWL_STYLE) & WS_CAPTION))
      return 2;
    POINT origin = {0, 0};
    RECT frame;
    ClientToScreen(window, &origin);
    GetWindowRect(window, &frame);
    return origin.y - frame.top <
                   GetSystemMetricsForDpi(SM_CYCAPTION, GetDpiForWindow(window))
               ? 1
               : 0;
  }
  case 1:
    if (!original)
      original = window;
    return original == window;
  case 2: {
    HRGN region = CreateRectRgn(0, 0, 0, 0);
    int kind = GetWindowRgn(window, region);
    RECT frame;
    GetWindowRect(window, &frame);
    ClientToScreen(window, &point);
    bool contains = kind == ERROR || PtInRegion(region, point.x - frame.left,
                                                point.y - frame.top);
    DeleteObject(region);
    return contains;
  }
  case 3:
    ClientToScreen(window, &point);
    return (double)SendMessageW(window, WM_NCHITTEST, 0,
                                MAKELPARAM(point.x, point.y));
  case 5:
    SendMessageW(window, WM_LBUTTONDOWN, MK_LBUTTON,
                 MAKELPARAM(point.x, point.y));
    return 1;
  case 6: {
    DWORD value = 0;
    return SUCCEEDED(
               DwmGetWindowAttribute(window, (DWORD)x, &value, sizeof(value)))
               ? (double)value
               : -1;
  }
  default:
    return -1;
  }
}
