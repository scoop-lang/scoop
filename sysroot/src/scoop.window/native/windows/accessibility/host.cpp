#include "provider.h"
#include <cmath>

namespace swax::win {
struct Request {
  const std::function<int32_t()> *body;
  int32_t result = 4;
};
bool WindowHost::onOwner() const { return GetCurrentThreadId() == thread; }
int32_t WindowHost::dispatch(const std::function<int32_t()> &body) {
  auto owner = tree.lock();
  if (!owner || owner->closed)
    return 4;
  if (onOwner())
    return body();
  Request request{&body};
  // The ordinary owner message loop executes this request. No private loop is
  // pumped.
  SendMessageW(hwnd, SW_ACCESSIBILITY, 0, reinterpret_cast<LPARAM>(&request));
  return request.result;
}
bool WindowHost::visible() const {
  return IsWindowVisible(hwnd) && !IsIconic(hwnd);
}
bool WindowHost::enabled() const { return IsWindowEnabled(hwnd); }
bool WindowHost::focused() const { return GetForegroundWindow() == hwnd; }
Rect WindowHost::bounds() const {
  RECT rect{};
  if (!GetClientRect(hwnd, &rect))
    return {};
  const double scale = GetDpiForWindow(hwnd) / 96.0;
  return scale > 0 ? Rect{0, 0, (rect.right - rect.left) / scale,
                          (rect.bottom - rect.top) / scale}
                   : Rect{};
}
std::optional<Rect> WindowHost::toScreen(Rect value) const {
  POINT origin{};
  double scale = GetDpiForWindow(hwnd) / 96.0;
  if (scale <= 0 || !ClientToScreen(hwnd, &origin))
    return {};
  return Rect{origin.x + value.x * scale, origin.y + value.y * scale,
              value.width * scale, value.height * scale};
}
std::optional<Point> WindowHost::fromScreen(Point value) const {
  POINT origin{};
  double scale = GetDpiForWindow(hwnd) / 96.0;
  if (scale <= 0 || !ClientToScreen(hwnd, &origin))
    return {};
  return Point{(value.x - origin.x) / scale, (value.y - origin.y) / scale};
}
void WindowHost::close() {
  if (window)
    window->accessibility = nullptr;
  window = nullptr;
  if (root) {
    UiaDisconnectProvider(root);
    root->Release();
    root = nullptr;
  }
}
} // namespace swax::win

extern "C" SWResult scoop_window_accessibility_create(
    void *pointer, SWAccessibilityCallback callback, void *context) {
  auto *window = static_cast<SWWindow *>(pointer);
  if (!window || window->destroying || window->accessibility)
    return {nullptr, -4};
  try {
    auto host = std::make_shared<swax::win::WindowHost>(window);
    auto tree = std::make_shared<swax::Tree>(host, callback, context);
    host->tree = tree;
    auto handle = std::make_unique<swax::Handle>(swax::Handle{tree});
    host->root = swax::win::simple(tree->actual(0));
    window->accessibility = handle.get();
    return {handle.release(), 0};
  } catch (const std::bad_alloc &) {
    return {nullptr, ERROR_NOT_ENOUGH_MEMORY};
  } catch (...) {
    return {nullptr, ERROR_INVALID_STATE};
  }
}
extern "C" bool sw_accessibility_message(SWWindow *window, UINT message,
                                         WPARAM wparam, LPARAM lparam,
                                         LRESULT *result) {
  auto *handle = static_cast<swax::Handle *>(window->accessibility);
  if (!handle)
    return false;
  auto tree = handle->tree;
  std::static_pointer_cast<swax::win::WindowHost>(tree->host)
      ->nativeChanged(message);
  if (message == WM_GETOBJECT && static_cast<LONG>(lparam) == UiaRootObjectId) {
    try {
      auto *provider =
          std::static_pointer_cast<swax::win::WindowHost>(tree->host)->root;
      if (!provider) {
        *result = 0;
        return true;
      }
      provider->AddRef();
      *result =
          UiaReturnRawElementProvider(window->hwnd, wparam, lparam, provider);
      provider->Release();
    } catch (...) {
      *result = 0;
      sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
    }
    return true;
  }
  if (message == SW_ACCESSIBILITY) {
    auto *request = reinterpret_cast<swax::win::Request *>(lparam);
    if (!tree->closed) {
      try {
        request->result = (*request->body)();
      } catch (...) {
        request->result = 4;
      }
    }
    *result = 0;
    return true;
  }
  return false;
}
extern "C" void sw_accessibility_destroy(SWWindow *window) {
  if (auto *handle = static_cast<swax::Handle *>(window->accessibility))
    handle->tree->close();
}
