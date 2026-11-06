#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <cmath>
#include <cstdint>
#include <dcomp.h>
#include <dwmapi.h>
#include <memory>
#include <new>
#include <wrl/client.h>

extern "C" bool scoop_window_test_surface_text(HWND, bool);
extern "C" bool scoop_window_test_surface_caret(HWND, double, double, double,
                                                double);

struct ViewProbe {
  HWND window;
  HWND parent;
  HWND firstChild;
  HWND child = nullptr;
  LONG_PTR procedure;
  Microsoft::WRL::ComPtr<IDCompositionDevice> composition;

  ~ViewProbe() {
    if (child && IsWindow(child))
      DestroyWindow(child);
  }
  bool matches() const {
    return IsWindow(window) && GetParent(window) == parent &&
           GetWindow(window, GW_CHILD) == firstChild &&
           GetWindowLongPtrW(window, GWLP_WNDPROC) == procedure;
  }
};

extern "C" {
int64_t scoop_surface_test_desktop_pixel(HWND window, double x, double y) {
  if (!IsWindow(window) || FAILED(DwmFlush()))
    return -1;
  const double scale = GetDpiForWindow(window) / 96.0;
  POINT point{static_cast<LONG>(std::round(x * scale)),
              static_cast<LONG>(std::round(y * scale))};
  if (!ClientToScreen(window, &point))
    return -1;
  HDC desktop = GetDC(nullptr);
  if (!desktop)
    return -1;
  const COLORREF pixel = GetPixel(desktop, point.x, point.y);
  ReleaseDC(nullptr, desktop);
  return pixel == CLR_INVALID ? -1LL : static_cast<int64_t>(pixel);
}

ViewProbe *scoop_surface_test_view_create(HWND window, bool customContent) {
  if (!IsWindow(window))
    return nullptr;
  auto probe = std::unique_ptr<ViewProbe>(new (std::nothrow) ViewProbe);
  if (!probe)
    return nullptr;
  probe->window = window;
  probe->parent = GetParent(window);
  probe->procedure = GetWindowLongPtrW(window, GWLP_WNDPROC);
  if (FAILED(
          DCompositionCreateDevice(nullptr, IID_PPV_ARGS(&probe->composition))))
    return nullptr;
  if (customContent) {
    probe->child =
        CreateWindowExW(0, L"STATIC", L"", WS_CHILD | WS_VISIBLE, 4, 4, 12, 12,
                        window, nullptr, GetModuleHandleW(nullptr), nullptr);
    if (!probe->child)
      return nullptr;
  }
  probe->firstChild = GetWindow(window, GW_CHILD);
  return probe.release();
}

bool scoop_surface_test_view_check(ViewProbe *probe, int32_t width,
                                   int32_t height, double scale) {
  RECT rect;
  return probe->matches() && GetClientRect(probe->window, &rect) &&
         rect.right == width && rect.bottom == height &&
         std::abs(GetDpiForWindow(probe->window) / 96.0 - scale) < 0.000001;
}

bool scoop_surface_test_view_restored(ViewProbe *probe) {
  if (!IsWindow(probe->window))
    return true;
  if (!probe->matches())
    return false;
  // The same HWND layer cannot have another target while Surface owns it.
  // A successful probe releases its empty target without committing a visual.
  Microsoft::WRL::ComPtr<IDCompositionTarget> target;
  return SUCCEEDED(
      probe->composition->CreateTargetForHwnd(probe->window, FALSE, &target));
}

void scoop_surface_test_view_drop(ViewProbe *probe) { delete probe; }

bool scoop_surface_test_view_button(ViewProbe *probe, double x, double y,
                                    bool pressed) {
  if (!IsWindow(probe->window))
    return false;
  const double scale = GetDpiForWindow(probe->window) / 96.0;
  const LPARAM position = MAKELPARAM(static_cast<short>(std::round(x * scale)),
                                     static_cast<short>(std::round(y * scale)));
  SendMessageW(probe->window, WM_MOUSEMOVE, 0, position);
  SendMessageW(probe->window, pressed ? WM_LBUTTONDOWN : WM_LBUTTONUP,
               pressed ? MK_LBUTTON : 0, position);
  return true;
}

bool scoop_surface_test_view_text(ViewProbe *probe, bool commit) {
  return IsWindow(probe->window) &&
         scoop_window_test_surface_text(probe->window, commit);
}

bool scoop_surface_test_view_caret(ViewProbe *probe, double x, double y,
                                   double width, double height) {
  return scoop_window_test_surface_caret(probe->window, x, y, width, height);
}
}
