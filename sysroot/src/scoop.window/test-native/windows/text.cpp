#include "../../native/windows/text_internal.h"

#include <cmath>
#include <functional>
#include <string>

namespace {
struct EditSession : ITfEditSession {
  explicit EditSession(std::function<HRESULT()> body) : body(std::move(body)) {}
  virtual ~EditSession() = default;
  std::function<HRESULT()> body;
  ULONG references = 1;

  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **object) override {
    if (!object)
      return E_POINTER;
    *object = nullptr;
    if (!IsEqualIID(iid, IID_IUnknown) && !IsEqualIID(iid, IID_ITfEditSession))
      return E_NOINTERFACE;
    *object = static_cast<ITfEditSession *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++references; }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG remaining = --references;
    if (!remaining)
      delete this;
    return remaining;
  }
  HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie) override { return body(); }
};

SWTextState *text_state(HWND hwnd) {
  SWWindow *window = (SWWindow *)GetWindowLongPtrW(hwnd, GWLP_USERDATA);
  return window ? static_cast<SWTextState *>(window->text) : nullptr;
}

HRESULT edit(HWND hwnd, bool write, const std::function<HRESULT()> &body) {
  SWTextState *state = text_state(hwnd);
  if (!state || !state->context)
    return E_UNEXPECTED;
  EditSession *session = new EditSession(body);
  HRESULT session_result = E_FAIL;
  DWORD flags = (write ? TF_ES_READWRITE : TF_ES_READ) | TF_ES_SYNC;
  HRESULT hr = state->context->RequestEditSession(0, session, flags,
                                                  &session_result);
  session->Release();
  return FAILED(hr) ? hr : session_result;
}

HRESULT async_edit(HWND hwnd, const std::function<HRESULT()> &body) {
  SWTextState *state = text_state(hwnd);
  if (!state || !state->context)
    return E_UNEXPECTED;
  bool ran = false;
  EditSession *session = new EditSession([&] {
    ran = true;
    return body();
  });
  HRESULT session_result = E_FAIL;
  HRESULT hr = state->context->RequestEditSession(0, session, TF_ES_READWRITE,
                                                  &session_result);
  if (SUCCEEDED(hr) && !ran)
    SendMessageW(hwnd, SW_TEXT_LOCK, 0, 0);
  session->Release();
  return FAILED(hr) || !ran ? E_FAIL : session_result;
}

HRESULT composition(HWND hwnd, bool start) {
  ITextStoreACP *store = sw_text_store_acp(text_state(hwnd));
  if (!store)
    return E_UNEXPECTED;
  ITfContextOwnerCompositionSink *sink = nullptr;
  HRESULT hr = store->QueryInterface(IID_ITfContextOwnerCompositionSink,
                                     (void **)&sink);
  if (FAILED(hr))
    return hr;
  if (start) {
    BOOL ok = FALSE;
    hr = sink->OnStartComposition(nullptr, &ok);
    if (SUCCEEDED(hr) && !ok)
      hr = E_FAIL;
  } else {
    hr = sink->OnEndComposition(nullptr);
  }
  sink->Release();
  return hr;
}

HRESULT replace(ITextStoreACP *store, LONG start, LONG end, const WCHAR *text,
                ULONG length) {
  TS_TEXTCHANGE change;
  return store->SetText(0, start, end, text, length, &change);
}

HRESULT document(ITextStoreACP *store, const std::wstring &expected,
                 LONG anchor, LONG cursor) {
  WCHAR actual[256];
  ULONG count = 0, runs = 0;
  LONG next = 0;
  HRESULT hr = store->GetText(0, -1, actual, 256, &count, nullptr, 0, &runs,
                              &next);
  if (FAILED(hr) || expected != std::wstring(actual, count))
    return E_FAIL;
  TS_SELECTION_ACP selection;
  ULONG fetched = 0;
  hr = store->GetSelection(TF_DEFAULT_SELECTION, 1, &selection, &fetched);
  if (FAILED(hr) || fetched != 1)
    return E_FAIL;
  LONG start = anchor < cursor ? anchor : cursor;
  LONG end = anchor < cursor ? cursor : anchor;
  return selection.acpStart == start && selection.acpEnd == end ? S_OK : E_FAIL;
}

HRESULT caret(ITextStoreACP *store, LONG cursor) {
  RECT rect;
  BOOL clipped = TRUE;
  HRESULT hr = store->GetTextExt(1, cursor, cursor, &rect, &clipped);
  if (FAILED(hr))
    return hr;
  POINT point = {(rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2};
  LONG acp = -1;
  hr = store->GetACPFromPoint(1, &point, 0, &acp);
  return SUCCEEDED(hr) && acp == cursor ? S_OK : E_FAIL;
}

HRESULT no_caret(ITextStoreACP *store) {
  RECT rect;
  BOOL clipped;
  return FAILED(store->GetTextExt(1, 0, 0, &rect, &clipped)) ? S_OK : E_FAIL;
}

HRESULT unavailable(ITextStoreACP *store) {
  WCHAR value[4];
  ULONG count, runs;
  LONG next;
  HRESULT hr = store->GetText(0, 1, value, 4, &count, nullptr, 0, &runs, &next);
  return FAILED(hr) && SUCCEEDED(no_caret(store)) ? S_OK : E_FAIL;
}

int32_t snapshot(HWND hwnd, int32_t phase) {
  ITextStoreACP *store = sw_text_store_acp(text_state(hwnd));
  if (!store)
    return 1;
  HRESULT hr = E_FAIL;
  switch (phase) {
  case 0:
    hr = edit(hwnd, false, [&] {
      HRESULT result = document(store, L"A😀中BC\nD", 6, 3);
      return FAILED(result) ? result : caret(store, 3);
    });
    break;
  case 1:
    hr = composition(hwnd, true);
    if (FAILED(hr))
      break;
    hr = edit(hwnd, true, [&] {
      HRESULT result = replace(store, 3, 6, L"仮", 1);
      if (FAILED(result))
        return result;
      result = replace(store, 3, 4, L"仮😀", 3);
      if (FAILED(result))
        return result;
      TS_SELECTION_ACP selection = {};
      selection.acpStart = 4;
      selection.acpEnd = 6;
      selection.style.ase = TS_AE_END;
      result = store->SetSelection(1, &selection);
      if (FAILED(result))
        return result;
      SWTextState *state = text_state(hwnd);
      if (state->projection.marked_start != 3 ||
          state->projection.marked_end != 6)
        return E_FAIL;
      return document(store, L"A😀仮😀\nD", 4, 6);
    });
    if (SUCCEEDED(hr))
      hr = composition(hwnd, false);
    break;
  case 2:
    hr = edit(hwnd, false, [&] {
      HRESULT result = document(store, L"A😀文日本語\nD", 7, 7);
      return FAILED(result) ? result : caret(store, 7);
    });
    break;
  case 3:
    hr = edit(hwnd, true, [&] { return replace(store, 3, 6, L"文", 1); });
    break;
  case 4:
    hr = composition(hwnd, true);
    if (FAILED(hr))
      break;
    hr = edit(hwnd, true, [&] { return replace(store, 4, 4, L"日本語", 3); });
    if (SUCCEEDED(hr))
      hr = composition(hwnd, false);
    break;
  case 5:
    hr = edit(hwnd, false, [&] {
      HRESULT result = document(store, L"external😀", 10, 10);
      return FAILED(result) ? result : caret(store, 10);
    });
    break;
  case 6:
    hr = edit(hwnd, false, [&] { return unavailable(store); });
    break;
  case 7:
    hr = edit(hwnd, false, [&] { return no_caret(store); });
    break;
  case 8:
    hr = composition(hwnd, true);
    if (SUCCEEDED(hr))
      hr = edit(hwnd, true, [&] { return replace(store, 7, 7, L"候補", 2); });
    break;
  case 9:
    hr = edit(hwnd, true, [&] {
      HRESULT result = replace(store, 7, 9, L"valid", 5);
      if (FAILED(result))
        return result;
      const WCHAR invalid[] = {0xd83d};
      return replace(store, 0, 0, invalid, 1);
    });
    if (FAILED(hr))
      hr = S_OK;
    else
      hr = E_FAIL;
    break;
  case 10:
    hr = edit(hwnd, true, [&] { return replace(store, 0, 1, L"R", 1); });
    break;
  default:
    return 2;
  }
  return FAILED(hr) ? 10 + phase : 0;
}
} // namespace

extern "C" {
int32_t scoop_window_test_snapshot(void *pointer, int32_t phase) {
  return snapshot(static_cast<HWND>(pointer), phase);
}

int32_t scoop_window_test_text(void *pointer) {
  HWND hwnd = static_cast<HWND>(pointer);
  ITextStoreACP *store = sw_text_store_acp(text_state(hwnd));
  if (!store)
    return 1;
  HRESULT hr = edit(hwnd, false, [&] {
    WCHAR value[4];
    ULONG count, runs;
    LONG next;
    if (SUCCEEDED(store->GetText(0, 1, value, 4, &count, nullptr, 0, &runs,
                                &next)))
      return E_FAIL;
    RECT rect;
    BOOL clipped;
    return store->GetTextExt(1, 0, 0, &rect, &clipped);
  });
  if (FAILED(hr))
    return 2;
  hr = composition(hwnd, true);
  if (SUCCEEDED(hr))
    hr = async_edit(hwnd, [&] { return replace(store, 0, 0, L"中😀", 3); });
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] { return replace(store, 1, 3, L"文", 1); });
  if (SUCCEEDED(hr))
    hr = composition(hwnd, false);
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"字", 1); });
  if (SUCCEEDED(hr))
    hr = composition(hwnd, true);
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"é😀", 3); });
  if (SUCCEEDED(hr))
    hr = composition(hwnd, false);
  if (SUCCEEDED(hr))
    hr = composition(hwnd, true);
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"old", 3); });
  if (SUCCEEDED(hr)) {
    hr = edit(hwnd, true, [&] {
      HRESULT result = replace(store, 0, 3, L"valid", 5);
      if (FAILED(result))
        return result;
      const WCHAR invalid[] = {0xd83d};
      return replace(store, 0, 0, invalid, 1);
    });
    if (FAILED(hr))
      hr = S_OK;
  }
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] {
      HRESULT result = replace(store, 0, 0, L"a", 1);
      if (FAILED(result))
        return result;
      result = replace(store, 1, 1, L"Q", 1);
      if (FAILED(result))
        return result;
      result = replace(store, 2, 2, L"d", 1);
      if (FAILED(result))
        return result;
      result = replace(store, 3, 3, L"", 0);
      if (FAILED(result))
        return result;
      return replace(store, 3, 3, L"", 0);
    });
  if (SUCCEEDED(hr))
    hr = composition(hwnd, true);
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"cancel", 6); });
  if (SUCCEEDED(hr)) {
    SWWindow *window = (SWWindow *)GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    sw_reset_input(window);
    hr = composition(hwnd, true);
  }
  if (SUCCEEDED(hr))
    hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"disable", 7); });
  return FAILED(hr) ? 3 : 0;
}

int32_t scoop_window_test_caret(HWND hwnd) {
  ITextStoreACP *store = sw_text_store_acp(text_state(hwnd));
  if (!store)
    return 0;
  HRESULT hr = edit(hwnd, false, [&] {
    RECT rect;
    BOOL clipped = TRUE;
    HRESULT result = store->GetTextExt(1, 0, 0, &rect, &clipped);
    if (FAILED(result))
      return result;
    double scale = GetDpiForWindow(hwnd) / 96.0;
    POINT first = {(LONG)std::lround(12 * scale), (LONG)std::lround(20 * scale)};
    POINT second = {(LONG)std::ceil(14 * scale), (LONG)std::ceil(38 * scale)};
    ClientToScreen(hwnd, &first);
    ClientToScreen(hwnd, &second);
    return rect.left == first.x && rect.top == first.y &&
                   rect.right == second.x && rect.bottom == second.y && !clipped
               ? S_OK
               : E_FAIL;
  });
  return SUCCEEDED(hr);
}

static bool surface_caret(HWND hwnd, bool available, double x, double y,
                          double width, double height, int64_t cursor,
                          bool query_point) {
  ITextStoreACP *store = sw_text_store_acp(text_state(hwnd));
  if (!store)
    return false;
  HRESULT hr = edit(hwnd, false, [&] {
    if (!available)
      return no_caret(store);
    RECT rect;
    BOOL clipped = TRUE;
    HRESULT result = store->GetTextExt(1, (LONG)cursor, (LONG)cursor, &rect,
                                       &clipped);
    if (FAILED(result))
      return result;
    double scale = GetDpiForWindow(hwnd) / 96.0;
    POINT first = {(LONG)std::lround(x * scale), (LONG)std::lround(y * scale)};
    POINT second = {(LONG)std::ceil((x + width) * scale),
                    (LONG)std::ceil((y + height) * scale)};
    ClientToScreen(hwnd, &first);
    ClientToScreen(hwnd, &second);
    if (rect.left != first.x || rect.top != first.y ||
        rect.right != second.x || rect.bottom != second.y || clipped)
      return E_FAIL;
    if (query_point) {
      POINT center = {(rect.left + rect.right) / 2,
                      (rect.top + rect.bottom) / 2};
      LONG acp = -1;
      result = store->GetACPFromPoint(1, &center, 0, &acp);
      if (FAILED(result) || acp != cursor)
        return E_FAIL;
    }
    return S_OK;
  });
  return SUCCEEDED(hr);
}

bool scoop_window_test_document_caret(HWND hwnd, bool available, double x,
                                      double y, double width, double height,
                                      int64_t cursor) {
  return surface_caret(hwnd, available, x, y, width, height, cursor, true);
}

bool scoop_window_test_surface_caret(HWND hwnd, double x, double y,
                                     double width, double height) {
  return surface_caret(hwnd, true, x, y, width, height, 0, false);
}

bool scoop_window_test_surface_text(HWND hwnd, bool commit) {
  SWTextState *state = text_state(hwnd);
  if (!state)
    return false;
  HRESULT hr = E_FAIL;
  if (commit && state->composing) {
    hr = composition(hwnd, false);
  } else if (commit) {
    ITextStoreACP *store = sw_text_store_acp(state);
    hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"A", 1); });
  } else {
    hr = composition(hwnd, true);
    if (SUCCEEDED(hr)) {
      ITextStoreACP *store = sw_text_store_acp(state);
      hr = edit(hwnd, true, [&] { return replace(store, 0, 0, L"中😀", 3); });
    }
  }
  return SUCCEEDED(hr);
}
} // extern "C"
