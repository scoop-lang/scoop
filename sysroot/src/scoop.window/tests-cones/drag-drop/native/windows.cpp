#include "../../../native/windows/drag_data.h"
#include <cstring>
#include <functional>
#include <memory>
#include <shellapi.h>
#include <shlobj.h>
#include <utility>

extern "C" SWResult scoop_window_drop_create(SWWindow *, SWDropCallback,
                                             void *);
extern "C" void scoop_window_drop_update(void *, int32_t);
extern "C" void scoop_window_drop_periodic(void *, int32_t);
extern "C" void scoop_window_drop_close(void *);

namespace {
struct Failure {
  int line;
};
#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      throw Failure{__LINE__};                                                 \
  } while (false)
struct Release {
  template <class T> void operator()(T *value) const { value->Release(); }
};
template <class T> using Com = std::unique_ptr<T, Release>;

std::vector<uint8_t> wide_bytes(const std::wstring &text, bool terminated) {
  const auto *bytes = reinterpret_cast<const uint8_t *>(text.data());
  return {bytes, bytes + (text.size() + terminated) * sizeof(WCHAR)};
}

std::vector<uint8_t> padded_text(const std::wstring &text) {
  auto bytes = wide_bytes(text, true);
  // HGLOBAL padding after the terminator is not part of the text value.
  bytes.insert(bytes.end(), {0x41, 0x00, 0xff});
  return bytes;
}

class Source final : public IDataObject {
  ULONG references = 1;
  Com<IDataObject> contents;
  void put(CLIPFORMAT format, const std::vector<uint8_t> &bytes) {
    CHECK(SUCCEEDED(
        sw_drag_set_bytes(contents.get(), format, bytes.data(), bytes.size())));
  }
  static bool payload(FORMATETC *type) {
    return type &&
           (type->cfFormat == CF_UNICODETEXT || type->cfFormat == CF_HDROP ||
            type->cfFormat == sw_drag_url_format());
  }

public:
  bool busy = false, unreadable = false;
  int queries = 0, busy_queries = 0, reads = 0;
  std::function<void()> on_query, on_read;
  explicit Source(int format) {
    IDataObject *object = nullptr;
    CHECK(SUCCEEDED(SHCreateDataObject(nullptr, 0, nullptr, nullptr,
                                       IID_PPV_ARGS(&object))));
    contents.reset(object);
    if (format == 3) {
      DROPFILES header = {};
      header.pFiles = sizeof(header);
      header.fWide = TRUE;
      std::vector<uint8_t> bytes(sizeof(header));
      memcpy(bytes.data(), &header, sizeof(header));
      for (const auto &path :
           {L"C:\\scoop-drop\\one 中文.txt", L"C:\\scoop-drop\\two 😀.txt"}) {
        auto value = wide_bytes(path, true);
        bytes.insert(bytes.end(), value.begin(), value.end());
      }
      bytes.resize(bytes.size() + sizeof(WCHAR), 0);
      put(CF_HDROP, bytes);
    } else {
      put(CF_UNICODETEXT, padded_text(L"中文😀\r\nvalue"));
      if (format == 2)
        put(sw_drag_url_format(),
            padded_text(L"https://example.test/path?q=1#sample"));
    }
  }
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override {
    if (!result)
      return E_POINTER;
    *result = nullptr;
    if (iid != IID_IUnknown && iid != IID_IDataObject)
      return E_NOINTERFACE;
    *result = static_cast<IDataObject *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++references; }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG left = --references;
    if (!left)
      delete this;
    return left;
  }
  HRESULT STDMETHODCALLTYPE QueryGetData(FORMATETC *format) override {
    if (!payload(format))
      return contents->QueryGetData(format);
    ++queries;
    if (busy) {
      ++busy_queries;
      return RPC_E_CALL_REJECTED;
    }
    if (on_query)
      std::exchange(on_query, {})();
    return contents->QueryGetData(format);
  }
  HRESULT STDMETHODCALLTYPE GetData(FORMATETC *format,
                                    STGMEDIUM *medium) override {
    if (!payload(format))
      return contents->GetData(format, medium);
    ++reads;
    if (on_read)
      std::exchange(on_read, {})();
    return unreadable ? DV_E_FORMATETC : contents->GetData(format, medium);
  }
  HRESULT STDMETHODCALLTYPE GetDataHere(FORMATETC *format,
                                        STGMEDIUM *medium) override {
    return contents->GetDataHere(format, medium);
  }
  HRESULT STDMETHODCALLTYPE GetCanonicalFormatEtc(FORMATETC *input,
                                                  FORMATETC *output) override {
    return contents->GetCanonicalFormatEtc(input, output);
  }
  HRESULT STDMETHODCALLTYPE SetData(FORMATETC *format, STGMEDIUM *medium,
                                    BOOL release) override {
    return contents->SetData(format, medium, release);
  }
  HRESULT STDMETHODCALLTYPE EnumFormatEtc(DWORD direction,
                                          IEnumFORMATETC **result) override {
    return contents->EnumFormatEtc(direction, result);
  }
  HRESULT STDMETHODCALLTYPE DAdvise(FORMATETC *format, DWORD flags,
                                    IAdviseSink *sink, DWORD *id) override {
    return contents->DAdvise(format, flags, sink, id);
  }
  HRESULT STDMETHODCALLTYPE DUnadvise(DWORD id) override {
    return contents->DUnadvise(id);
  }
  HRESULT STDMETHODCALLTYPE EnumDAdvise(IEnumSTATDATA **result) override {
    return contents->EnumDAdvise(result);
  }
};

struct Receiver {
  int format = 1, offers = 0, drops = 0, leaves = 0, formats = 0;
  std::vector<std::vector<uint8_t>> items;
  static int32_t call(void *context, int32_t phase, int32_t formats,
                      int32_t effects, int32_t, double, double,
                      int64_t modifiers, int32_t, void *payload) {
    auto &self = *static_cast<Receiver *>(context);
    if (phase == 0) {
      ++self.offers;
      self.formats = formats;
      int effect = (modifiers & 2) ? 1 : (effects & 2) ? 2 : 1;
      return self.format * 256 + effect;
    }
    if (phase == 1) {
      ++self.drops;
      self.items = static_cast<SWDropPayload *>(payload)->items;
      return 1;
    }
    ++self.leaves;
    return 0;
  }
};

struct Target {
  Receiver receiver;
  void *owned = nullptr;
  Com<IDropTarget> target;
  explicit Target(SWWindow *window) {
    SWResult value =
        scoop_window_drop_create(window, Receiver::call, &receiver);
    CHECK(!value.error && value.pointer);
    owned = value.pointer;
    target.reset(static_cast<IDropTarget *>(owned));
    target->AddRef();
    scoop_window_drop_periodic(owned, 1);
  }
  void close() {
    if (owned)
      scoop_window_drop_close(std::exchange(owned, nullptr));
  }
  ~Target() { close(); }
};

void stable_formats(SWWindow *window, POINTL point) {
  Target destination(window);
  for (int format = 1; format <= 3; ++format) {
    Com<Source> source(new Source(format));
    auto &receiver = destination.receiver;
    receiver.format = format;
    DWORD effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
    CHECK(destination.target->DragEnter(source.get(), MK_CONTROL, point,
                                        &effect) == S_OK);
    CHECK(effect == DROPEFFECT_COPY && source->reads == 0);
    CHECK(receiver.formats == (format == 2 ? 3 : format == 3 ? 4 : 1));
    int queries = source->queries;
    CHECK(queries > 0);
    source->busy = true;
    for (int update = 0; update < 12; ++update) {
      effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
      POINTL moved = {point.x + update, point.y + update};
      CHECK(destination.target->DragOver(MK_CONTROL, moved, &effect) == S_OK);
      CHECK(effect == DROPEFFECT_COPY);
      int offers = receiver.offers;
      CHECK(sw_drop_message(window, WM_TIMER,
                            reinterpret_cast<WPARAM>(destination.owned)));
      CHECK(receiver.offers == offers + 1);
    }
    effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
    CHECK(destination.target->DragOver(MK_SHIFT, point, &effect) == S_OK);
    CHECK(effect == DROPEFFECT_MOVE);
    effect = DROPEFFECT_MOVE;
    CHECK(destination.target->DragOver(MK_CONTROL, point, &effect) == S_OK);
    CHECK(effect == DROPEFFECT_NONE);
    CHECK(source->queries == queries && source->busy_queries == 0);
    CHECK(source->reads == 0);
    effect = DROPEFFECT_COPY | DROPEFFECT_MOVE;
    CHECK(destination.target->Drop(source.get(), MK_CONTROL, point, &effect) ==
          S_OK);
    CHECK(effect == DROPEFFECT_COPY && source->reads == 1);
    CHECK(receiver.drops == format && receiver.leaves == format);
    if (format == 3) {
      CHECK(receiver.items.size() == 2);
      CHECK(receiver.items[0] ==
            wide_bytes(L"C:\\scoop-drop\\one 中文.txt", false));
      CHECK(receiver.items[1] ==
            wide_bytes(L"C:\\scoop-drop\\two 😀.txt", false));
    } else {
      CHECK(receiver.items.size() == 1);
      std::string expected = format == 1
                                 ? "中文😀\r\nvalue"
                                 : "https://example.test/path?q=1#sample";
      CHECK(std::string(receiver.items[0].begin(), receiver.items[0].end()) ==
            expected);
    }
    CHECK(source->queries == queries && source->busy_queries == 0);
    effect = DROPEFFECT_COPY;
    CHECK(destination.target->DragOver(0, point, &effect) == S_OK);
    CHECK(effect == DROPEFFECT_NONE);
  }
}

void shell_preview(SWWindow *window, POINTL point) {
  Target destination(window);
  Com<Source> source(new Source(1));
  std::vector<uint8_t> rgba(32 * 16 * 4, 0);
  for (size_t i = 0; i < rgba.size(); i += 4) {
    rgba[i] = 227;
    rgba[i + 3] = 130;
  }
  CHECK(sw_drag_set_preview(source.get(), 144, 32, 16, rgba.data(), 16, 8, 2.5,
                            6) == S_OK);
  // The Shell helper publishes its image independently of the text payload.
  FORMATETC image = {
      static_cast<CLIPFORMAT>(RegisterClipboardFormatW(L"DragImageBits")),
      nullptr, DVASPECT_CONTENT, -1, TYMED_HGLOBAL};
  STGMEDIUM medium = {};
  CHECK(source->GetData(&image, &medium) == S_OK);
  CHECK(medium.tymed == TYMED_HGLOBAL &&
        GlobalSize(medium.hGlobal) >= 24 * 12 * 4);
  ReleaseStgMedium(&medium);
  DWORD effect = DROPEFFECT_COPY;
  CHECK(destination.target->DragEnter(source.get(), MK_CONTROL, point,
                                      &effect) == S_OK);
  CHECK(effect == DROPEFFECT_COPY && !source->reads);
  CHECK(destination.target->DragOver(MK_CONTROL, point, &effect) == S_OK);
  CHECK(destination.target->Drop(source.get(), MK_CONTROL, point, &effect) ==
        S_OK);
  CHECK(effect == DROPEFFECT_COPY && source->reads == 1);
}

void invalidated_data(SWWindow *window, POINTL point) {
  for (int scenario = 0; scenario < 4; ++scenario) {
    Target destination(window);
    Com<Source> source(new Source(1));
    if (scenario == 1)
      source->on_query = [&] { destination.close(); };
    if (scenario == 2)
      source->on_query = [&] {
        scoop_window_drop_update(destination.owned, 0);
      };
    DWORD effect = DROPEFFECT_COPY;
    CHECK(destination.target->DragEnter(source.get(), MK_CONTROL, point,
                                        &effect) == S_OK);
    CHECK(effect ==
          (scenario == 1 || scenario == 2 ? DROPEFFECT_NONE : DROPEFFECT_COPY));
    CHECK(source->reads == 0);
    if (scenario == 0)
      source->unreadable = true;
    if (scenario == 3)
      source->on_read = [&] { destination.close(); };
    effect = DROPEFFECT_COPY;
    CHECK(destination.target->Drop(source.get(), MK_CONTROL, point, &effect) ==
          S_OK);
    CHECK(effect == DROPEFFECT_NONE && destination.receiver.drops == 0);
    CHECK(source->reads == (scenario == 0 || scenario == 3 ? 1 : 0));
    CHECK(destination.target->DragLeave() == S_OK);
    CHECK(destination.receiver.leaves ==
          (scenario == 0 || scenario == 2 ? 1 : 0));
    destination.close();
    effect = DROPEFFECT_COPY;
    CHECK(destination.target->DragEnter(source.get(), MK_CONTROL, point,
                                        &effect) == S_OK);
    CHECK(effect == DROPEFFECT_NONE);
  }
}

void updated_receiver(SWWindow *window, POINTL point) {
  Target destination(window);
  Com<Source> source(new Source(1));
  source->on_query = [&] { scoop_window_drop_update(destination.owned, 1); };
  DWORD effect = DROPEFFECT_COPY;
  CHECK(destination.target->DragEnter(source.get(), MK_CONTROL, point,
                                      &effect) == S_OK);
  CHECK(effect == DROPEFFECT_NONE && destination.receiver.offers == 0);
  source->busy = true;
  effect = DROPEFFECT_COPY;
  CHECK(destination.target->DragOver(MK_CONTROL, point, &effect) == S_OK);
  CHECK(effect == DROPEFFECT_COPY);
  effect = DROPEFFECT_COPY;
  CHECK(destination.target->Drop(source.get(), MK_CONTROL, point, &effect) ==
        S_OK);
  CHECK(effect == DROPEFFECT_COPY && destination.receiver.drops == 1);
  CHECK(source->reads == 1 && source->busy_queries == 0);
}
} // namespace

extern "C" int32_t scoop_drop_test_windows_metadata(HWND hwnd) {
  try {
    auto *window =
        reinterpret_cast<SWWindow *>(GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    CHECK(window && window->hwnd == hwnd && !window->drop_target);
    POINT point = {35, 45};
    CHECK(IsWindowVisible(hwnd) && ClientToScreen(hwnd, &point));
    shell_preview(window, {point.x, point.y});
    stable_formats(window, {point.x, point.y});
    invalidated_data(window, {point.x, point.y});
    updated_receiver(window, {point.x, point.y});
    CHECK(!window->drop_target);
    return 0;
  } catch (const Failure &failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}
