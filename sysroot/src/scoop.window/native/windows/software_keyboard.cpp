#include "../software_keyboard.h"
#include "keyboard_abi.h"
#include "text_internal.h"
#include <algorithm>
#include <new>
#include <roapi.h>
#include <shobjidl.h>
#include <winstring.h>

namespace {
struct Keyboard;
void publish(Keyboard *keyboard);

class PaneHandler final : public IFrameworkInputPaneHandler {
  volatile LONG references = 1;

public:
  SWWindow *window;
  explicit PaneHandler(SWWindow *window) : window(window) {}
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override {
    if (!result)
      return E_POINTER;
    *result = nullptr;
    if (IsEqualIID(iid, IID_IUnknown) ||
        IsEqualIID(iid, IID_IFrameworkInputPaneHandler)) {
      *result = static_cast<IFrameworkInputPaneHandler *>(this);
      AddRef();
      return S_OK;
    }
    return E_NOINTERFACE;
  }
  ULONG STDMETHODCALLTYPE AddRef() override {
    return InterlockedIncrement(&references);
  }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG left = InterlockedDecrement(&references);
    if (!left)
      delete this;
    return left;
  }
  HRESULT STDMETHODCALLTYPE Showing(RECT *rect, BOOL ensure_visible) override;
  HRESULT STDMETHODCALLTYPE Hiding(BOOL ensure_visible) override;
};

struct Keyboard {
  SWWindow *owner;
  IFrameworkInputPane *pane = nullptr;
  PaneHandler *handler = nullptr;
  SWInputPane2 *control = nullptr;
  DWORD cookie = 0;
  bool advised = false, known = false, shown = false, control_checked = false;
  RECT screen = {};
  SWSoftwareKeyboard current = {};
};

void publish(Keyboard *keyboard) {
  SWWindow *window = keyboard->owner;
  if (!window || window->destroying)
    return;
  SWSoftwareKeyboard value = {};
  if (keyboard->known) {
    value.state = keyboard->shown ? 2 : 1;
    if (keyboard->shown) {
      POINT start = {keyboard->screen.left, keyboard->screen.top};
      POINT end = {keyboard->screen.right, keyboard->screen.bottom};
      RECT bounds;
      if (ScreenToClient(window->hwnd, &start) &&
          ScreenToClient(window->hwnd, &end) &&
          GetClientRect(window->hwnd, &bounds)) {
        LONG left = std::max(start.x, bounds.left),
             top = std::max(start.y, bounds.top);
        LONG right = std::min(end.x, bounds.right),
             bottom = std::min(end.y, bounds.bottom);
        double scale = GetDpiForWindow(window->hwnd) / 96.0;
        if (right > left && bottom > top && scale > 0) {
          value = {3, left / scale, top / scale, (right - left) / scale,
                   (bottom - top) / scale};
        }
      }
    }
  }
  const auto &previous = keyboard->current;
  if (value.state == previous.state && value.x == previous.x &&
      value.y == previous.y && value.width == previous.width &&
      value.height == previous.height)
    return;
  keyboard->current = value;
  SWEvent *event = sw_event(window, 22);
  if (event) {
    event->data.flags = value.state;
    event->data.x = value.x;
    event->data.y = value.y;
    event->data.z = value.width;
    event->data.w = value.height;
    sw_enqueue(window->application, event);
  }
}

HRESULT PaneHandler::Showing(RECT *rect, BOOL) {
  if (!rect)
    return E_POINTER;
  if (window && window->keyboard) {
    auto *keyboard = static_cast<Keyboard *>(window->keyboard);
    keyboard->known = keyboard->shown = true;
    keyboard->screen = *rect;
    publish(keyboard);
  }
  return S_OK;
}
HRESULT PaneHandler::Hiding(BOOL) {
  if (window && window->keyboard) {
    auto *keyboard = static_cast<Keyboard *>(window->keyboard);
    keyboard->known = true;
    keyboard->shown = false;
    publish(keyboard);
  }
  return S_OK;
}

HRESULT load_control(Keyboard *keyboard) {
  if (keyboard->control_checked)
    return keyboard->control ? S_OK : E_NOINTERFACE;
  keyboard->control_checked = true;
  static const WCHAR name[] = L"Windows.UI.ViewManagement.InputPane";
  HSTRING runtime_class = nullptr;
  HRESULT status =
      WindowsCreateString(name, ARRAYSIZE(name) - 1, &runtime_class);
  if (FAILED(status))
    return status;
  SWInputPaneInterop *factory = nullptr;
  status = RoGetActivationFactory(runtime_class, sw_input_pane_interop,
                                  reinterpret_cast<void **>(&factory));
  WindowsDeleteString(runtime_class);
  if (FAILED(status))
    return status;
  status = factory->GetForWindow(keyboard->owner->hwnd, sw_input_pane2,
                                 reinterpret_cast<void **>(&keyboard->control));
  factory->Release();
  return status;
}
} // namespace

extern "C" int64_t sw_keyboard_start(SWWindow *window) {
  if (window->keyboard)
    return 0;
  auto *keyboard = new (std::nothrow) Keyboard;
  if (!keyboard)
    return ERROR_NOT_ENOUGH_MEMORY;
  keyboard->owner = window;
  keyboard->handler = new (std::nothrow) PaneHandler(window);
  if (!keyboard->handler) {
    delete keyboard;
    return ERROR_NOT_ENOUGH_MEMORY;
  }
  window->keyboard = keyboard;
  HRESULT status = CoCreateInstance(
      CLSID_FrameworkInputPane, nullptr, CLSCTX_INPROC_SERVER,
      IID_IFrameworkInputPane, reinterpret_cast<void **>(&keyboard->pane));
  if (SUCCEEDED(status)) {
    status = keyboard->pane->AdviseWithHWND(window->hwnd, keyboard->handler,
                                            &keyboard->cookie);
    keyboard->advised = SUCCEEDED(status);
    RECT rect = {};
    if (keyboard->advised && SUCCEEDED(keyboard->pane->Location(&rect))) {
      keyboard->screen = rect;
      keyboard->known = true;
      keyboard->shown = !IsRectEmpty(&rect);
      publish(keyboard);
    }
  }
  return status == E_OUTOFMEMORY ? ERROR_NOT_ENOUGH_MEMORY : 0;
}
extern "C" void sw_keyboard_geometry(SWWindow *window) {
  if (window->keyboard)
    publish(static_cast<Keyboard *>(window->keyboard));
}
extern "C" void sw_keyboard_destroy(SWWindow *window) {
  auto *keyboard = static_cast<Keyboard *>(window->keyboard);
  if (!keyboard)
    return;
  window->keyboard = nullptr;
  keyboard->owner = nullptr;
  keyboard->handler->window = nullptr;
  if (keyboard->advised)
    keyboard->pane->Unadvise(keyboard->cookie);
  if (keyboard->pane)
    keyboard->pane->Release();
  if (keyboard->control)
    keyboard->control->Release();
  keyboard->handler->Release();
  delete keyboard;
}
extern "C" SWSoftwareKeyboard scoop_window_keyboard_state(SWWindow *window) {
  auto *keyboard = static_cast<Keyboard *>(window->keyboard);
  return keyboard ? keyboard->current : SWSoftwareKeyboard{};
}
extern "C" int64_t scoop_window_keyboard_request(SWWindow *window,
                                                 int64_t connection,
                                                 int32_t visible, void *) {
  auto *text = static_cast<SWTextState *>(window->text);
  if (!window->ime_allowed || !text || text->closed ||
      text->connection != connection || !text->focused || window->destroying ||
      window->input_blocked || !IsWindowVisible(window->hwnd))
    return 1;
  int64_t error = sw_keyboard_start(window);
  if (error)
    return error;
  auto *keyboard = static_cast<Keyboard *>(window->keyboard);
  HRESULT status = load_control(keyboard);
  if (FAILED(status))
    return status == E_OUTOFMEMORY ? ERROR_NOT_ENOUGH_MEMORY : -1;
  boolean accepted = false;
  status = visible ? keyboard->control->TryShow(&accepted)
                   : keyboard->control->TryHide(&accepted);
  if (status == E_OUTOFMEMORY)
    return ERROR_NOT_ENOUGH_MEMORY;
  return SUCCEEDED(status) && accepted ? 0 : 1;
}
