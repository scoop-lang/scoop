#include "text.h"
#include <algorithm>
#include <cmath>

namespace swax::win {
HRESULT readText(const ElementPtr &element,
                 std::shared_ptr<const State> &result) {
  result = element->state();
  if (!result || !element->exposed())
    return UIA_E_ELEMENTNOTAVAILABLE;
  if (!publicText(*result))
    return UIA_E_NOTSUPPORTED;
  if (result->document) {
    const auto document = sw_text_document_state(result->document->pointer);
    if (!document.alive || document.revision != result->documentRevision ||
        document.length != static_cast<int64_t>(result->text[3]->size()))
      return UIA_E_ELEMENTNOTAVAILABLE;
  }
  return S_OK;
}
HRESULT textGeometryResult(int32_t status) {
  if (status == 1 || status == 0)
    return S_OK;
  return status == -2 ? E_OUTOFMEMORY : UIA_E_ELEMENTNOTAVAILABLE;
}
SAFEARRAY *rangeArray(ITextRangeProvider *range) {
  auto result = SafeArrayCreateVector(VT_UNKNOWN, 0, range ? 1 : 0);
  if (!result) {
    if (range)
      range->Release();
    throw std::bad_alloc();
  }
  LONG index = 0;
  HRESULT status = range ? SafeArrayPutElement(result, &index, range) : S_OK;
  if (range)
    range->Release();
  if (FAILED(status)) {
    SafeArrayDestroy(result);
    throw std::bad_alloc();
  }
  return result;
}
IUnknown *textProvider(const ElementPtr &element) {
  return static_cast<ITextProvider2 *>(new TextProvider(element));
}
static ITextRangeProvider *
snapshotRange(const ElementPtr &element,
              const std::shared_ptr<const State> &state, int64_t start,
              int64_t end, int32_t first = 1, int32_t last = 0) {
  if (start < 0 || end < start ||
      end > static_cast<int64_t>(state->text[3]->size()))
    return nullptr;
  return new TextRange(element,
                       {state, start, end, first, start == end ? first : last});
}
ITextRangeProvider *textRange(const ElementPtr &element, int64_t start,
                              int64_t end, int32_t first, int32_t last) {
  std::shared_ptr<const State> state;
  if (FAILED(readText(element, state)))
    return nullptr;
  return snapshotRange(element, state, start, end, first, last);
}

TextProvider::TextProvider(ElementPtr value)
    : element(std::move(value)), tree(element->tree.lock()) {
  const auto status = CoCreateFreeThreadedMarshaler(
      static_cast<ITextProvider2 *>(this), &marshaler);
  if (FAILED(status))
    throw ComFailure{status};
}
TextProvider::~TextProvider() {
  if (marshaler)
    marshaler->Release();
}
ULONG TextProvider::AddRef() { return ++references; }
ULONG TextProvider::Release() {
  ULONG left = --references;
  if (!left)
    delete this;
  return left;
}
HRESULT TextProvider::QueryInterface(REFIID iid, void **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (iid == IID_IUnknown || iid == __uuidof(ITextProvider) ||
      iid == __uuidof(ITextProvider2))
    *result = static_cast<ITextProvider2 *>(this);
  else if (iid == IID_IMarshal && marshaler)
    return marshaler->QueryInterface(iid, result);
  else
    return E_NOINTERFACE;
  AddRef();
  return S_OK;
}
HRESULT TextProvider::GetSelection(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (FAILED(status))
      return status;
    if (state->data.anchor < 0 || state->data.cursor < 0) {
      *result = rangeArray(nullptr);
      return S_OK;
    }
    int32_t affinity = state->data.flags & SWAXCursorUpstream ? 0 : 1;
    auto range = snapshotRange(
        element, state, std::min(state->data.anchor, state->data.cursor),
        std::max(state->data.anchor, state->data.cursor), affinity);
    if (!range)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = rangeArray(range);
    return S_OK;
  });
}
HRESULT TextProvider::GetVisibleRanges(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (FAILED(status))
      return status;
    if (!state->textGeometry)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto &layout = state->textGeometry->data;
    auto visible = layout.visible_range(layout.context);
    if (visible.status != 1) {
      if (visible.status == 0)
        *result = rangeArray(nullptr);
      return textGeometryResult(visible.status);
    }
    auto range = snapshotRange(element, state, visible.start, visible.end);
    if (!range)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = rangeArray(range);
    return S_OK;
  });
}
HRESULT TextProvider::RangeFromChild(IRawElementProviderSimple *child,
                                     ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (FAILED(status))
      return status;
    auto target = fromProvider(tree, child);
    if (!target)
      return E_INVALIDARG;
    while (target && target != element) {
      for (const auto &entry : state->textChildren) {
        if (target->reference.kind == 0 &&
            target->reference.node == entry.node) {
          *result = snapshotRange(element, state, entry.start, entry.end);
          return *result ? S_OK : UIA_E_ELEMENTNOTAVAILABLE;
        }
      }
      target = target->parent();
    }
    return E_INVALIDARG;
  });
}
HRESULT TextProvider::RangeFromPoint(UiaPoint point,
                                     ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!std::isfinite(point.x) || !std::isfinite(point.y))
      return E_INVALIDARG;
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (FAILED(status))
      return status;
    if (!state->textGeometry)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto position = tree->host->fromScreen({point.x, point.y});
    if (!position)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto &layout = state->textGeometry->data;
    auto range =
        layout.range_at_point(layout.context, position->x, position->y);
    if (range.status != 1)
      return range.status == 0 ? UIA_E_ELEMENTNOTAVAILABLE
                               : textGeometryResult(range.status);
    *result = snapshotRange(element, state, range.start, range.start);
    return *result ? S_OK : UIA_E_ELEMENTNOTAVAILABLE;
  });
}
HRESULT TextProvider::get_DocumentRange(ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (FAILED(status))
      return status;
    *result = snapshotRange(element, state, 0,
                            static_cast<int64_t>(state->text[3]->size()));
    return *result ? S_OK : UIA_E_ELEMENTNOTAVAILABLE;
  });
}
HRESULT
TextProvider::get_SupportedTextSelection(SupportedTextSelection *result) {
  if (!result)
    return E_POINTER;
  *result = SupportedTextSelection_None;
  return guarded([&]() -> HRESULT {
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (SUCCEEDED(status) && (state->data.actions & 16))
      *result = SupportedTextSelection_Single;
    return status;
  });
}
HRESULT TextProvider::RangeFromAnnotation(IRawElementProviderSimple *,
                                          ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return UIA_E_NOTSUPPORTED;
}
HRESULT TextProvider::GetCaretRange(BOOL *active, ITextRangeProvider **result) {
  if (!active || !result)
    return E_POINTER;
  *active = FALSE;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    std::shared_ptr<const State> state;
    auto status = readText(element, state);
    if (FAILED(status))
      return status;
    if (state->data.cursor < 0)
      return UIA_E_NOTSUPPORTED;
    *result =
        snapshotRange(element, state, state->data.cursor, state->data.cursor,
                      state->data.flags & SWAXCursorUpstream ? 0 : 1);
    if (!*result)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto snapshot = tree->snapshot();
    *active = snapshot && snapshot->focused == state->data.number &&
              tree->host->focused();
    return S_OK;
  });
}
} // namespace swax::win
