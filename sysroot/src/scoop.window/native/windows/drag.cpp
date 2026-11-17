#include "drag_data.h"
#include "input/delivery.h"
#include <cstring>
#include <new>
#include <shellapi.h>

namespace {
class DragSource final : public IDropSource {
  ULONG references = 1;

public:
  SWWindow *owner;
  SWApplication *application;
  SWDragData *data = new SWDragData;
  SWDragCallback callback;
  void *context;
  int effects, initial_effects, format = 0, feedback = 0;
  int64_t contact;
  bool direct, ready = false, waiting = false, active = false, closed = false;
  std::vector<std::wstring> paths;

  DragSource(SWWindow *window, SWNativeInput *input, int effects,
             SWDragCallback callback, void *context)
      : owner(window), application(window->application), callback(callback),
        context(context), effects(effects), initial_effects(effects),
        contact(input->contact), direct(input->kind == SWContact) {}
  ~DragSource() { data->Release(); }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override {
    if (!result)
      return E_POINTER;
    *result = nullptr;
    if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_IDropSource)) {
      *result = static_cast<IDropSource *>(this);
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
  HRESULT STDMETHODCALLTYPE QueryContinueDrag(BOOL escape,
                                              DWORD keys) override {
    if (escape || closed || !available() || !effects)
      return DRAGDROP_S_CANCEL;
    return (keys & MK_LBUTTON)    ? S_OK
           : (feedback & effects) ? DRAGDROP_S_DROP
                                  : DRAGDROP_S_CANCEL;
  }
  HRESULT STDMETHODCALLTYPE GiveFeedback(DWORD effect) override {
    feedback = static_cast<int>(effect & 7);
    if ((effect & 7) && !(effect & effects)) {
      SetCursor(LoadCursorW(nullptr, IDC_NO));
      return S_OK;
    }
    return DRAGDROP_S_USEDEFAULTCURSORS;
  }
  bool available() const {
    return owner && !owner->destroying && !owner->input_blocked &&
           IsWindowVisible(owner->hwnd) && !application->stopped;
  }
  bool contact_live() const {
    return owner && sw_contact_find(owner->input.contacts, contact) &&
           owner->input.contacts && !owner->input.contacts->next;
  }
  void finish(int effect) {
    waiting = ready = active = false;
    if (application->drag_source == this)
      application->drag_source = nullptr;
    if (callback)
      callback(context, 1, effect);
  }
  int64_t start() {
    if (closed || !ready || active || !available())
      return -4;
    if (direct && !contact_live()) {
      finish(0);
      return 0;
    }
    if (direct && application->promoted_drag_window != owner->hwnd) {
      waiting = true;
      return 0;
    }
    AddRef();
    waiting = ready = false;
    active = true;
    if (GetCapture() == owner->hwnd)
      ReleaseCapture();
    if (callback)
      callback(context, 0, 0);
    DWORD result = 0;
    HRESULT status =
        closed ? DRAGDROP_S_CANCEL
               : DoDragDrop(data, this, static_cast<DWORD>(effects), &result);
    int effect = status == DRAGDROP_S_DROP ? static_cast<int>(result & 7) : 0;
    if ((effect != 1 && effect != 2 && effect != 4) ||
        !(initial_effects & effect))
      effect = 0;
    if (SUCCEEDED(status))
      finish(effect);
    else {
      active = false;
      if (application->drag_source == this)
        application->drag_source = nullptr;
    }
    Release();
    return FAILED(status) ? static_cast<uint32_t>(status) : 0;
  }
};

bool mouse_live(SWWindow *window, SWNativeInput *input) {
  return input->kind == SWPointer && input->button == 0 &&
         (window->input.pressed_buttons & 1);
}

void add_wide(SWDragData *data, CLIPFORMAT type, const std::wstring &value) {
  const auto *bytes = reinterpret_cast<const uint8_t *>(value.c_str());
  data->formats.push_back(
      {type, {bytes, bytes + (value.size() + 1) * sizeof(WCHAR)}});
}
} // namespace

extern "C" SWResult scoop_window_drag_create(SWWindow *window,
                                             SWNativeInput *input,
                                             int32_t effects,
                                             SWDragCallback callback,
                                             void *context) {
  if (!callback || effects <= 0 || effects > 7)
    return {nullptr, -8};
  if (!input || input->window != window->identity ||
      (!mouse_live(window, input) &&
       !(input->kind == SWContact &&
         sw_contact_find(window->input.contacts, input->contact))))
    return {nullptr, -4};
  if (window->application->drag_source)
    return {nullptr, -5};
  try {
    auto *source = new DragSource(window, input, effects, callback, context);
    if (!source->available() || (source->direct && !source->contact_live())) {
      source->Release();
      return {nullptr, -4};
    }
    return {source, 0};
  } catch (const std::bad_alloc &) {
    return {nullptr, ERROR_NOT_ENOUGH_MEMORY};
  }
}

extern "C" int64_t scoop_window_drag_add(DragSource *source, int32_t format,
                                         const uint8_t *bytes, int64_t length) {
  if (source->closed || source->ready || source->active)
    return -4;
  if (source->format || (format != 1 && format != 2) || length < 0 ||
      (length && (!bytes || memchr(bytes, 0, static_cast<size_t>(length)))))
    return -8;
  WCHAR *wide = sw_wide(bytes, length);
  if (!wide)
    return -8;
  int64_t status = 0;
  try {
    std::wstring text(wide);
    if (format == 2 && !sw_drag_absolute_url(text))
      status = -8;
    else {
      add_wide(source->data, CF_UNICODETEXT, text);
      if (format == 2) {
        CLIPFORMAT type = sw_drag_url_format();
        if (!type)
          status = GetLastError();
        else
          add_wide(source->data, type, text);
      }
      source->format = format;
    }
  } catch (const std::bad_alloc &) {
    status = ERROR_NOT_ENOUGH_MEMORY;
  }
  free(wide);
  return status;
}

extern "C" int64_t scoop_window_drag_add_path(DragSource *source,
                                              const uint8_t *bytes,
                                              int64_t length) {
  if (source->closed || source->ready || source->active)
    return -4;
  if ((source->format && source->format != 3) || !bytes || length <= 0 ||
      length % sizeof(WCHAR) || static_cast<uint64_t>(length) > SIZE_MAX)
    return -8;
  const WCHAR *path = reinterpret_cast<const WCHAR *>(bytes);
  size_t count = static_cast<size_t>(length) / sizeof(WCHAR);
  for (size_t n = 0; n < count; ++n)
    if (!path[n])
      return -8;
  try {
    source->paths.emplace_back(path, count);
    source->format = 3;
    return 0;
  } catch (const std::bad_alloc &) {
    return ERROR_NOT_ENOUGH_MEMORY;
  }
}

extern "C" int64_t scoop_window_drag_ready(DragSource *source) {
  if (source->closed || source->ready || source->active || !source->available())
    return -4;
  if (source->application->drag_source)
    return -5;
  if (!source->format)
    return -8;
  try {
    if (source->format == 3) {
      std::vector<uint8_t> bytes(sizeof(DROPFILES), 0);
      for (const auto &path : source->paths) {
        const auto *value = reinterpret_cast<const uint8_t *>(path.c_str());
        bytes.insert(bytes.end(), value,
                     value + (path.size() + 1) * sizeof(WCHAR));
      }
      bytes.resize(bytes.size() + sizeof(WCHAR), 0);
      DROPFILES header = {};
      header.pFiles = sizeof(DROPFILES);
      header.fWide = TRUE;
      memcpy(bytes.data(), &header, sizeof(header));
      source->data->formats.push_back({CF_HDROP, std::move(bytes)});
    }
    source->ready = true;
    source->application->drag_source = source;
    return 0;
  } catch (const std::bad_alloc &) {
    return ERROR_NOT_ENOUGH_MEMORY;
  }
}
extern "C" int64_t scoop_window_drag_start(DragSource *source) {
  return source->start();
}
extern "C" int64_t scoop_window_drag_effects(DragSource *source,
                                             int32_t effects) {
  if (source->closed || (!source->ready && !source->active))
    return -4;
  if (effects < 0 || (effects & ~source->initial_effects))
    return -8;
  source->effects = effects;
  return 0;
}
extern "C" void scoop_window_drag_close(DragSource *source) {
  source->closed = true;
  source->callback = nullptr;
  source->context = nullptr;
  if (!source->active && source->application->drag_source == source)
    source->application->drag_source = nullptr;
  source->Release();
}

int sw_drag_local(SWWindow *window, IDataObject *data) {
  auto *source = static_cast<DragSource *>(window->application->drag_source);
  if (!source || source->closed)
    return 0;
  source->AddRef();
  IUnknown *identity = nullptr;
  HRESULT status =
      data->QueryInterface(IID_IUnknown, reinterpret_cast<void **>(&identity));
  bool same = SUCCEEDED(status) && identity && !source->closed &&
              identity == static_cast<IDataObject *>(source->data);
  if (identity)
    identity->Release();
  source->Release();
  return same ? 3 : 0;
}

extern "C" void sw_drag_contact_changed(SWWindow *window) {
  auto *source = static_cast<DragSource *>(window->application->drag_source);
  if (source && source->owner == window && source->direct && !source->active &&
      !source->contact_live())
    source->finish(0);
}

extern "C" void sw_drag_promoted_mouse(SWWindow *window, UINT message,
                                       WPARAM keys) {
  SWApplication *application = window->application;
  if (application->compat_delivery || application->stopped ||
      (message != WM_MOUSEMOVE && message != WM_LBUTTONDOWN) ||
      !(keys & MK_LBUTTON))
    return;
  auto *running = static_cast<DragSource *>(application->drag_source);
  if (running && running->active)
    return;
  HWND hwnd = window->hwnd;
  application->compat_delivery = true;
  application->promoted_drag_window = hwnd;
  // Deliver queued contacts through the existing application boundary while
  // still on the real compatibility mouse message's stack, as OLE requires.
  application->callback(application->context, 0, 0);
  auto *source = static_cast<DragSource *>(application->drag_source);
  if (source && source->waiting && source->owner &&
      source->owner->hwnd == hwnd) {
    source->AddRef();
    int64_t error = source->start();
    if (error && source->callback)
      source->callback(source->context, 2, static_cast<int32_t>(error));
    source->Release();
  }
  application->promoted_drag_window = nullptr;
  application->compat_delivery = false;
}

extern "C" void sw_drag_window_destroy(SWWindow *window) {
  auto *source = static_cast<DragSource *>(window->application->drag_source);
  if (source && source->owner == window) {
    source->owner = nullptr;
    if (!source->active)
      source->finish(0);
  }
}
