#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <math.h>
#include <stdint.h>
#include <windows.h>

int32_t scoop_test_ui_region(HWND window, void *view, int32_t field, double x,
                             double y) {
  double scale = GetDpiForWindow(window) / 96.0;
  POINT point = {(LONG)round(x * scale), (LONG)round(y * scale)};
  ClientToScreen(window, &point);
  if (field == 0)
    return SendMessageW(window, WM_NCHITTEST, 0,
                        MAKELPARAM(point.x, point.y)) == HTCAPTION;
  if (field != 1)
    return -1;
  HRGN region = CreateRectRgn(0, 0, 0, 0);
  int kind = GetWindowRgn(window, region);
  RECT bounds;
  GetWindowRect(window, &bounds);
  int result = kind == ERROR ||
               PtInRegion(region, point.x - bounds.left, point.y - bounds.top);
  DeleteObject(region);
  return result;
}
