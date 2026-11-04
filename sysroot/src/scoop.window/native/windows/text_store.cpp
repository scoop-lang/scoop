#include "text_internal.h"

#include <algorithm>
#include <utility>

namespace {
constexpr TsViewCookie VIEW_COOKIE = 1;

template <typename F> HRESULT guarded(F &&body) noexcept {
  try {
    return body();
  } catch (const std::bad_alloc &) {
    return E_OUTOFMEMORY;
  } catch (...) {
    return E_FAIL;
  }
}
} // namespace

struct SWTextStore : ITextStoreACP, ITfContextOwnerCompositionSink {
  explicit SWTextStore(SWTextState *state) : state(state) {}
  virtual ~SWTextStore() = default;

  SWTextState *state;
  ULONG references = 1;
  ITextStoreACPSink *sink = nullptr;
  IUnknown *sink_identity = nullptr;
  DWORD sink_mask = 0;
  DWORD lock = 0;
  DWORD pending_flags = 0;
  bool pending = false;

  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid,
                                           void **object) override {
    if (!object)
      return E_POINTER;
    *object = nullptr;
    if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITextStoreACP))
      *object = static_cast<ITextStoreACP *>(this);
    else if (IsEqualIID(iid, IID_ITfContextOwnerCompositionSink))
      *object = static_cast<ITfContextOwnerCompositionSink *>(this);
    else
      return E_NOINTERFACE;
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

  HRESULT STDMETHODCALLTYPE AdviseSink(REFIID iid, IUnknown *object,
                                       DWORD mask) override {
    if (!object || !IsEqualIID(iid, IID_ITextStoreACPSink))
      return E_INVALIDARG;
    if (sink)
      return E_FAIL;
    ITextStoreACPSink *next = nullptr;
    HRESULT hr = object->QueryInterface(IID_ITextStoreACPSink, (void **)&next);
    if (FAILED(hr))
      return hr;
    IUnknown *identity = nullptr;
    hr = object->QueryInterface(IID_IUnknown, (void **)&identity);
    if (FAILED(hr)) {
      next->Release();
      return hr;
    }
    sink = next;
    sink_identity = identity;
    sink_mask = mask;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE UnadviseSink(IUnknown *object) override {
    if (!object || !sink_identity)
      return E_INVALIDARG;
    IUnknown *identity = nullptr;
    HRESULT hr = object->QueryInterface(IID_IUnknown, (void **)&identity);
    if (FAILED(hr))
      return hr;
    identity->Release();
    if (identity != sink_identity)
      return E_INVALIDARG;
    sink->Release();
    sink_identity->Release();
    sink = nullptr;
    sink_identity = nullptr;
    sink_mask = 0;
    return S_OK;
  }

  HRESULT grant(DWORD flags) {
    return guarded([&] {
      if (!sink || state->closed)
        return TF_E_DISCONNECTED;
      bool write = (flags & TS_LF_READWRITE) != 0;
      if (write && !sw_text_begin_edit(state))
        return TF_E_NOOBJECT;
      lock = flags;
      HRESULT hr = sink->OnLockGranted(flags);
      lock = 0;
      if (write)
        sw_text_end_edit(state, FAILED(hr));
      if (pending)
        PostMessageW(state->window->hwnd, SW_TEXT_LOCK, 0, 0);
      return hr;
    });
  }

  HRESULT STDMETHODCALLTYPE RequestLock(DWORD flags,
                                        HRESULT *session) override {
    return guarded([&] {
      if (!session || !sink || !(flags & (TS_LF_READ | TS_LF_READWRITE)))
        return E_INVALIDARG;
      *session = S_OK;
      DWORD requested = flags & (TS_LF_READ | TS_LF_READWRITE);
      if (flags & TS_LF_SYNC) {
        if (lock)
          return TF_E_SYNCHRONOUS;
        *session = grant(requested);
        return S_OK;
      }
      if (pending)
        requested = (pending_flags & TS_LF_READWRITE) || (flags & TS_LF_READWRITE)
                        ? TS_LF_READWRITE
                        : TS_LF_READ;
      pending = true;
      pending_flags = requested;
      if (!PostMessageW(state->window->hwnd, SW_TEXT_LOCK, 0, 0))
        return HRESULT_FROM_WIN32(GetLastError());
      *session = TF_S_ASYNC;
      return S_OK;
    });
  }

  HRESULT STDMETHODCALLTYPE GetStatus(TS_STATUS *status) override {
    if (!status)
      return E_POINTER;
    status->dwDynamicFlags = 0;
    status->dwStaticFlags = TS_SS_NOHIDDENTEXT | TS_SS_MULTILINE;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE QueryInsert(LONG start, LONG end, ULONG count,
                                        LONG *result_start,
                                        LONG *result_end) override {
    return guarded([&] {
      if (!result_start || !result_end || !(lock & TS_LF_READ))
        return TF_E_NOLOCK;
      const SWTextDocument *document = sw_text_query(state);
      if (!document)
        return TF_E_NOOBJECT;
      if (!sw_text_range(document->text, start, end) || count > LONG_MAX ||
          (int64_t)end + count > LONG_MAX)
        return TF_E_INVALIDPOS;
      *result_start = start;
      *result_end = end + (LONG)count;
      return S_OK;
    });
  }

  HRESULT STDMETHODCALLTYPE GetSelection(ULONG index, ULONG count,
                                         TS_SELECTION_ACP *selection,
                                         ULONG *fetched) override {
    if (fetched)
      *fetched = 0;
    if (!selection || !fetched || !(lock & TS_LF_READ))
      return TF_E_NOLOCK;
    if (index == TF_DEFAULT_SELECTION)
      index = 0;
    if (index != 0 || count == 0)
      return TF_E_NOSELECTION;
    const SWTextDocument *document = sw_text_query(state);
    if (!document ||
        (!document->present && !state->composing && !state->editing) ||
        (document->present && state->applied != state->sequence &&
         !state->editing))
      return TF_E_NOSELECTION;
    LONG start = std::min(document->anchor, document->cursor);
    LONG end = std::max(document->anchor, document->cursor);
    if (!sw_text_range(document->text, start, end))
      return TF_E_NOSELECTION;
    selection[0].acpStart = start;
    selection[0].acpEnd = end;
    selection[0].style.ase = document->cursor >= document->anchor
                                 ? TS_AE_END
                                 : TS_AE_START;
    selection[0].style.fInterimChar = state->composing;
    *fetched = 1;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE SetSelection(ULONG count,
                                         const TS_SELECTION_ACP *selection) override {
    if (!(lock & TS_LF_READWRITE))
      return TF_E_NOLOCK;
    if (count != 1 || !selection)
      return E_NOTIMPL;
    HRESULT hr = sw_text_select(state, selection[0]);
    if (SUCCEEDED(hr))
      sw_text_store_notify_selection(state);
    return hr;
  }

  HRESULT STDMETHODCALLTYPE GetText(LONG start, LONG end, WCHAR *plain,
                                    ULONG plain_required, ULONG *plain_count,
                                    TS_RUNINFO *run_info, ULONG run_required,
                                    ULONG *run_count, LONG *next) override {
    if (plain_count)
      *plain_count = 0;
    if (run_count)
      *run_count = 0;
    if (next)
      *next = start;
    if (!(lock & TS_LF_READ))
      return TF_E_NOLOCK;
    if ((!plain && plain_required) || (!run_info && run_required) || !next)
      return E_INVALIDARG;
    const SWTextDocument *document = sw_text_query(state);
    if (!document ||
        (!document->present && !state->composing && !state->editing))
      return TF_E_NOOBJECT;
    if (end == -1)
      end = (LONG)document->text.size();
    if (!sw_text_range(document->text, start, end))
      return TF_E_INVALIDPOS;
    ULONG length = (ULONG)(end - start);
    ULONG copied = std::min(length, plain_required);
    if (copied)
      std::copy_n(document->text.data() + start, copied, plain);
    if (plain_count)
      *plain_count = copied;
    ULONG runs = 0;
    if (run_required && length) {
      run_info[0].uCount = length;
      run_info[0].type = TS_RT_PLAIN;
      runs = 1;
    }
    if (run_count)
      *run_count = runs;
    *next = start + (LONG)std::max(copied, runs ? length : 0);
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE SetText(DWORD flags, LONG start, LONG end,
                                    const WCHAR *text, ULONG length,
                                    TS_TEXTCHANGE *change) override {
    return guarded([&] {
      if (!(lock & TS_LF_READWRITE) || !change)
        return TF_E_NOLOCK;
      HRESULT hr =
          sw_text_replace(state, start, end, text, length, true, change);
      if (SUCCEEDED(hr)) {
        sw_text_store_notify_change(state, *change);
        sw_text_store_notify_selection(state);
      }
      return hr;
    });
  }

  HRESULT STDMETHODCALLTYPE GetFormattedText(LONG, LONG,
                                             IDataObject **object) override {
    if (object)
      *object = nullptr;
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE GetEmbedded(LONG, REFGUID, REFIID,
                                        IUnknown **object) override {
    if (object)
      *object = nullptr;
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE QueryInsertEmbedded(const GUID *, const FORMATETC *,
                                                BOOL *insertable) override {
    if (!insertable)
      return E_POINTER;
    *insertable = FALSE;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE InsertEmbedded(DWORD, LONG, LONG, IDataObject *,
                                           TS_TEXTCHANGE *) override {
    return E_NOTIMPL;
  }

  HRESULT STDMETHODCALLTYPE InsertTextAtSelection(DWORD flags, const WCHAR *text,
                                                  ULONG length, LONG *start,
                                                  LONG *end,
                                                  TS_TEXTCHANGE *change) override {
    return guarded([&] {
      if (!(lock & TS_LF_READWRITE) || !start || !end || !change)
        return TF_E_NOLOCK;
      const SWTextDocument &document = state->projection;
      LONG first = std::min(document.anchor, document.cursor);
      LONG last = std::max(document.anchor, document.cursor);
      HRESULT hr =
          sw_text_replace(state, first, last, text, length, false, change);
      if (FAILED(hr))
        return hr;
      *start = change->acpStart;
      *end = change->acpNewEnd;
      sw_text_store_notify_change(state, *change);
      sw_text_store_notify_selection(state);
      return S_OK;
    });
  }

  HRESULT STDMETHODCALLTYPE InsertEmbeddedAtSelection(DWORD, IDataObject *,
                                                      LONG *, LONG *,
                                                      TS_TEXTCHANGE *) override {
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE RequestSupportedAttrs(DWORD, ULONG,
                                                  const TS_ATTRID *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RequestAttrsAtPosition(LONG position, ULONG,
                                                   const TS_ATTRID *,
                                                   DWORD) override {
    const SWTextDocument *document = sw_text_query(state);
    if (!(lock & TS_LF_READ))
      return TF_E_NOLOCK;
    if (!document)
      return TF_E_NOOBJECT;
    return sw_text_boundary(document->text, position) ? S_OK : TF_E_INVALIDPOS;
  }
  HRESULT STDMETHODCALLTYPE RequestAttrsTransitioningAtPosition(
      LONG position, ULONG count, const TS_ATTRID *attributes,
      DWORD flags) override {
    return RequestAttrsAtPosition(position, count, attributes, flags);
  }
  HRESULT STDMETHODCALLTYPE FindNextAttrTransition(
      LONG start, LONG halt, ULONG, const TS_ATTRID *, DWORD, LONG *next,
      BOOL *found, LONG *offset) override {
    if (!next || !found || !offset)
      return E_POINTER;
    *next = halt;
    *found = FALSE;
    *offset = 0;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE RetrieveRequestedAttrs(ULONG, TS_ATTRVAL *,
                                                   ULONG *fetched) override {
    if (!fetched)
      return E_POINTER;
    *fetched = 0;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE GetEndACP(LONG *end) override {
    if (!end || !(lock & TS_LF_READ))
      return TF_E_NOLOCK;
    const SWTextDocument *document = sw_text_query(state);
    if (!document ||
        (!document->present && !state->composing && !state->editing))
      return TF_E_NOOBJECT;
    *end = (LONG)document->text.size();
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetActiveView(TsViewCookie *view) override {
    if (!view)
      return E_POINTER;
    *view = VIEW_COOKIE;
    return S_OK;
  }

  HRESULT caret_rect(RECT *rect) {
    if (!rect || !state->caret_known)
      return TF_E_INVALIDPOINT;
    const SWTextDocument *document = sw_text_query(state);
    if (!document ||
        (document->present && state->caret_revision != document->revision))
      return TF_E_INVALIDPOINT;
    double scale = GetDpiForWindow(state->window->hwnd) / 96.0;
    POINT first = {(LONG)std::lround(state->caret_x * scale),
                   (LONG)std::lround(state->caret_y * scale)};
    POINT second = {
        (LONG)std::ceil((state->caret_x + state->caret_width) * scale),
        (LONG)std::ceil((state->caret_y + state->caret_height) * scale)};
    if (!ClientToScreen(state->window->hwnd, &first) ||
        !ClientToScreen(state->window->hwnd, &second))
      return HRESULT_FROM_WIN32(GetLastError());
    rect->left = first.x;
    rect->top = first.y;
    rect->right = second.x;
    rect->bottom = second.y;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE GetACPFromPoint(TsViewCookie view,
                                            const POINT *point, DWORD flags,
                                            LONG *acp) override {
    if (view != VIEW_COOKIE || !point || !acp || !(lock & TS_LF_READ))
      return TF_E_NOLOCK;
    const SWTextDocument *document = sw_text_query(state);
    if (!document || !document->present)
      return TF_E_INVALIDPOINT;
    RECT rect;
    HRESULT hr = caret_rect(&rect);
    if (FAILED(hr))
      return hr;
    POINT value = *point;
    if (!PtInRect(&rect, value))
      return TF_E_INVALIDPOINT;
    *acp = document->cursor;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE GetTextExt(TsViewCookie view, LONG start, LONG end,
                                       RECT *rect, BOOL *clipped) override {
    if (view != VIEW_COOKIE || !rect || !clipped || !(lock & TS_LF_READ))
      return TF_E_NOLOCK;
    const SWTextDocument *document = sw_text_query(state);
    if (!document)
      return TF_E_INVALIDPOINT;
    if (document->present &&
        (!sw_text_range(document->text, start, end) ||
         document->cursor < start || document->cursor > end))
      return TF_E_INVALIDPOS;
    HRESULT hr = caret_rect(rect);
    *clipped = FALSE;
    return hr;
  }

  HRESULT STDMETHODCALLTYPE GetScreenExt(TsViewCookie view, RECT *rect) override {
    if (view != VIEW_COOKIE || !rect)
      return E_INVALIDARG;
    if (!GetClientRect(state->window->hwnd, rect))
      return HRESULT_FROM_WIN32(GetLastError());
    POINT origin = {rect->left, rect->top};
    POINT end = {rect->right, rect->bottom};
    if (!ClientToScreen(state->window->hwnd, &origin) ||
        !ClientToScreen(state->window->hwnd, &end))
      return HRESULT_FROM_WIN32(GetLastError());
    *rect = {origin.x, origin.y, end.x, end.y};
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetWnd(TsViewCookie view, HWND *hwnd) override {
    if (view != VIEW_COOKIE || !hwnd)
      return E_INVALIDARG;
    *hwnd = state->window->hwnd;
    return S_OK;
  }

  HRESULT STDMETHODCALLTYPE OnStartComposition(ITfCompositionView *,
                                               BOOL *ok) override {
    if (!ok)
      return E_POINTER;
    *ok = !state->closed && !state->composing && !state->canceling &&
          (!state->document.present || state->applied == state->sequence);
    if (*ok)
      state->composing = true;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnUpdateComposition(ITfCompositionView *,
                                                ITfRange *) override {
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE OnEndComposition(ITfCompositionView *) override {
    return guarded([&] {
      sw_text_end_composition(state);
      return S_OK;
    });
  }

  void notify_change(const TS_TEXTCHANGE &change) {
    if (sink && (sink_mask & TS_AS_TEXT_CHANGE))
      sink->OnTextChange(0, &change);
  }
  void notify_selection() {
    if (sink && (sink_mask & TS_AS_SEL_CHANGE))
      sink->OnSelectionChange();
  }
  void notify_layout() {
    if (sink && (sink_mask & TS_AS_LAYOUT_CHANGE))
      sink->OnLayoutChange(TS_LC_CHANGE, VIEW_COOKIE);
  }
};

HRESULT sw_text_store_create(SWTextState *state, TfClientId client) {
  return guarded([&] {
    SWTextStore *store = new (std::nothrow) SWTextStore(state);
    if (!store)
      return E_OUTOFMEMORY;
    state->store = store;
    HRESULT hr = state->window->application->text_manager
                     ? S_OK
                     : E_UNEXPECTED;
    ITfThreadMgr *thread = nullptr;
    if (SUCCEEDED(hr)) {
      thread = static_cast<SWTextManager *>(state->window->application
                                                ->text_manager)
                   ->thread;
      hr = thread->CreateDocumentMgr(&state->document_manager);
    }
    if (SUCCEEDED(hr))
      hr = state->document_manager->CreateContext(
          client, 0, static_cast<ITextStoreACP *>(store), &state->context,
          &state->edit_cookie);
    if (SUCCEEDED(hr))
      hr = state->document_manager->Push(state->context);
    if (FAILED(hr)) {
      sw_text_store_close(state);
      return hr;
    }
    return S_OK;
  });
}

ITextStoreACP *sw_text_store_acp(SWTextState *state) {
  return state && state->store ? static_cast<ITextStoreACP *>(state->store)
                               : nullptr;
}

void sw_text_store_close(SWTextState *state) {
  if (!state->store)
    return;
  if (state->context) {
    ITfContextOwnerCompositionServices *compositions = nullptr;
    if (SUCCEEDED(state->context->QueryInterface(
            IID_ITfContextOwnerCompositionServices, (void **)&compositions))) {
      compositions->TerminateComposition(NULL);
      compositions->Release();
    }
  }
  if (state->document_manager) {
    state->document_manager->Pop(TF_POPF_ALL);
    state->document_manager->Release();
    state->document_manager = nullptr;
  }
  if (state->context) {
    state->context->Release();
    state->context = nullptr;
  }
  SWTextStore *store = state->store;
  state->store = nullptr;
  if (store->sink)
    store->sink->Release();
  if (store->sink_identity)
    store->sink_identity->Release();
  store->sink = nullptr;
  store->sink_identity = nullptr;
  store->Release();
}

void sw_text_store_terminate_composition(SWTextState *state) {
  if (!state->context)
    return;
  ITfContextOwnerCompositionServices *compositions = nullptr;
  if (SUCCEEDED(state->context->QueryInterface(
          IID_ITfContextOwnerCompositionServices, (void **)&compositions))) {
    compositions->TerminateComposition(NULL);
    compositions->Release();
  }
}

void sw_text_store_grant_pending(SWTextState *state) {
  SWTextStore *store = state ? state->store : nullptr;
  if (!store || !store->pending || store->lock)
    return;
  DWORD flags = store->pending_flags;
  store->pending = false;
  store->grant(flags);
}

void sw_text_store_grant_pending(SWWindow *window) {
  sw_text_store_grant_pending(static_cast<SWTextState *>(window->text));
}

void sw_text_store_notify_change(SWTextState *state,
                                 const TS_TEXTCHANGE &change) {
  if (state->store)
    state->store->notify_change(change);
}
void sw_text_store_notify_selection(SWTextState *state) {
  if (state->store)
    state->store->notify_selection();
}
void sw_text_notify_layout(SWTextState *state) {
  if (state && state->store)
    state->store->notify_layout();
}
