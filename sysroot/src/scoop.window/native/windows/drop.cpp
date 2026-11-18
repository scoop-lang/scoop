#include "drag_data.h"
#include <new>

namespace {
class DropTarget final : public IDropTarget {
  ULONG references = 1;

public:
  SWWindow *owner;
  HWND hwnd;
  SWDropCallback callback;
  void *context;
  IDataObject *data = nullptr;
  bool enabled = true, closed = false, periodic = false, in_callback = false;
  bool receiving = false, registered = false, inside = false;
  uint64_t revision = 0;
  DWORD keys = 0, effects = 0;
  POINTL position = {};
  int choice = 0, formats = 0;

  DropTarget(SWWindow *window, SWDropCallback callback, void *context)
      : owner(window), hwnd(window->hwnd), callback(callback),
        context(context) {}
  ~DropTarget() {
    if (data)
      data->Release();
  }
  struct Hold {
    DropTarget *target;
    explicit Hold(DropTarget *value) : target(value) { target->AddRef(); }
    ~Hold() { target->Release(); }
  };
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override {
    if (!result)
      return E_POINTER;
    *result = nullptr;
    if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_IDropTarget)) {
      *result = static_cast<IDropTarget *>(this);
      AddRef();
      return S_OK;
    }
    return E_NOINTERFACE;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++references; }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG left = --references;
    if (!left)
      delete this;
    return left;
  }
  bool available() const {
    return owner && !closed && enabled && callback && !owner->destroying &&
           !owner->input_blocked && IsWindowVisible(hwnd);
  }
  void timer() {
    KillTimer(hwnd, reinterpret_cast<UINT_PTR>(this));
    if (periodic && inside && available() && !receiving)
      if (!SetTimer(hwnd, reinterpret_cast<UINT_PTR>(this), 16, nullptr))
        sw_fail(owner->application, GetLastError());
  }
  int call(int phase, int selection, SWDropPayload *payload,
           int *offered = nullptr) {
    if (!callback || closed || in_callback)
      return 0;
    POINT point = {position.x, position.y};
    if (phase != 2 && (!owner || !ScreenToClient(hwnd, &point)))
      return 0;
    in_callback = true;
    if (offered)
      *offered = formats;
    int local = data && owner ? sw_drag_local(owner, data) : 0;
    if (closed || !callback) {
      in_callback = false;
      return 0;
    }
    double scale = owner ? GetDpiForWindow(hwnd) / 96.0 : 1;
    if (scale <= 0) {
      in_callback = false;
      return 0;
    }
    // OLE supplies Control/Shift with the offer; preserve the other current
    // modifiers from the same owner thread.
    uint64_t modifiers = sw_modifiers();
    modifiers = (modifiers & ~UINT64_C(3)) | ((keys & MK_SHIFT) ? 1 : 0) |
                ((keys & MK_CONTROL) ? 2 : 0);
    in_callback = true;
    int result =
        callback(context, phase, formats, static_cast<int>(effects & 7), local,
                 point.x / scale, point.y / scale,
                 static_cast<int64_t>(modifiers), selection, payload);
    in_callback = false;
    return result;
  }
  int query() {
    if (!inside || !data || !available() || in_callback || receiving)
      return 0;
    uint64_t current = revision;
    int offered = 0;
    int value = call(0, 0, nullptr, &offered);
    return current == revision && inside && data && available()
               ? sw_drop_choice(value, offered, static_cast<int>(effects & 7))
               : 0;
  }
  void finish() {
    bool notify = inside;
    inside = false;
    choice = 0;
    formats = 0;
    KillTimer(hwnd, reinterpret_cast<UINT_PTR>(this));
    IDataObject *previous = data;
    data = nullptr;
    if (notify)
      call(2, 0, nullptr);
    if (previous)
      previous->Release();
  }
  HRESULT STDMETHODCALLTYPE DragEnter(IDataObject *object, DWORD state,
                                      POINTL point, DWORD *effect) override {
    Hold hold(this);
    if (!effect || !object)
      return E_POINTER;
    DWORD offered = *effect;
    *effect = DROPEFFECT_NONE;
    if (in_callback || receiving || !available())
      return S_OK;
    finish();
    if (!available())
      return S_OK;
    data = object;
    data->AddRef();
    inside = true;
    effects = offered;
    keys = state;
    position = point;
    uint64_t current = revision;
    // Querying a foreign IDataObject can pump incoming OLE calls. Keep that
    // work at entry; hover and timer updates only renegotiate local metadata.
    in_callback = true;
    int declared = sw_drag_formats(object);
    in_callback = false;
    if (!inside || data != object || closed)
      return S_OK;
    formats = declared;
    if (current != revision || !available())
      return S_OK;
    choice = query();
    *effect = choice & 255;
    timer();
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE DragOver(DWORD state, POINTL point,
                                     DWORD *effect) override {
    Hold hold(this);
    if (!effect)
      return E_POINTER;
    effects = *effect;
    *effect = DROPEFFECT_NONE;
    keys = state;
    position = point;
    choice = query();
    *effect = choice & 255;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE DragLeave() override {
    Hold hold(this);
    if (!in_callback && !receiving)
      finish();
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE Drop(IDataObject *object, DWORD state, POINTL point,
                                 DWORD *effect) override {
    Hold hold(this);
    if (!effect || !object)
      return E_POINTER;
    effects = *effect;
    *effect = DROPEFFECT_NONE;
    if (receiving || in_callback)
      return S_OK;
    keys = state;
    position = point;
    int selected = query();
    if (!selected) {
      finish();
      return S_OK;
    }
    receiving = true;
    KillTimer(hwnd, reinterpret_cast<UINT_PTR>(this));
    uint64_t current = revision;
    object->AddRef();
    SWDropPayload payload;
    bool read = sw_drag_read(object, selected / 256, payload);
    object->Release();
    if (read && current == revision && inside && available() &&
        call(1, selected, &payload))
      *effect = selected & 255;
    receiving = false;
    finish();
    return S_OK;
  }
  void close() {
    if (closed)
      return;
    closed = true;
    callback = nullptr;
    context = nullptr;
    ++revision;
    KillTimer(hwnd, reinterpret_cast<UINT_PTR>(this));
    if (owner && owner->drop_target == this)
      owner->drop_target = nullptr;
    owner = nullptr;
    if (registered) {
      registered = false;
      RevokeDragDrop(hwnd);
    }
    finish();
  }
};
} // namespace

extern "C" SWResult scoop_window_drop_create(SWWindow *window,
                                             SWDropCallback callback,
                                             void *context) {
  if (!callback || window->destroying)
    return {nullptr, -4};
  if (window->drop_target)
    return {nullptr, -5};
  auto *target = new (std::nothrow) DropTarget(window, callback, context);
  if (!target)
    return {nullptr, ERROR_NOT_ENOUGH_MEMORY};
  HRESULT status = RegisterDragDrop(window->hwnd, target);
  if (FAILED(status)) {
    target->Release();
    return {nullptr, static_cast<uint32_t>(status)};
  }
  target->registered = true;
  window->drop_target = target;
  return {target, 0};
}
extern "C" void scoop_window_drop_update(DropTarget *target, int32_t enabled) {
  ++target->revision;
  target->enabled = enabled != 0;
  target->choice = 0;
  target->timer();
}
extern "C" void scoop_window_drop_periodic(DropTarget *target,
                                           int32_t enabled) {
  target->periodic = enabled != 0;
  target->timer();
}
extern "C" void scoop_window_drop_leave(DropTarget *target) {
  DropTarget::Hold hold(target);
  target->finish();
}
extern "C" void scoop_window_drop_close(DropTarget *target) {
  target->close();
  target->Release();
}
extern "C" bool sw_drop_message(SWWindow *window, UINT message, WPARAM key) {
  auto *target = static_cast<DropTarget *>(window->drop_target);
  if (!target || message != WM_TIMER ||
      key != reinterpret_cast<UINT_PTR>(target))
    return false;
  DropTarget::Hold hold(target);
  if (!target->periodic || !target->inside || !target->available() ||
      target->receiving) {
    KillTimer(target->hwnd, reinterpret_cast<UINT_PTR>(target));
    return true;
  }
  if (target->inside && target->available() && !target->receiving) {
    POINT point;
    if (GetCursorPos(&point)) {
      target->position = {point.x, point.y};
      target->choice = target->query();
    }
  }
  return true;
}
extern "C" void sw_drop_window_destroy(SWWindow *window) {
  auto *target = static_cast<DropTarget *>(window->drop_target);
  if (target)
    target->close();
}
