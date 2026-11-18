#include "text.h"
#include <algorithm>
#include <climits>
#include <stdexcept>

namespace swax::win {
static bool byteBoundary(const std::string &text, int64_t offset) {
  return offset >= 0 && offset <= static_cast<int64_t>(text.size()) &&
         (offset == static_cast<int64_t>(text.size()) ||
          (static_cast<uint8_t>(text[offset]) & 0xc0) != 0x80);
}
TextRange::TextRange(ElementPtr value, const TextView &view)
    : element(std::move(value)), tree(element->tree.lock()),
      document(view.state->document), epoch(view.state->documentEpoch),
      textRevision(view.state->data.text_revision), start(view.start),
      end(view.end), first(view.first), last(view.last) {
  if (document) {
    anchor =
        sw_text_range_create(document->pointer, view.state->documentRevision,
                             start, end, first, last);
    if (!anchor) {
      auto current = sw_text_document_state(document->pointer);
      if (!current.alive || current.revision != view.state->documentRevision)
        throw ComFailure{static_cast<HRESULT>(UIA_E_ELEMENTNOTAVAILABLE)};
      throw std::bad_alloc();
    }
  }
  const auto status = CoCreateFreeThreadedMarshaler(
      static_cast<ITextRangeProvider2 *>(this), &marshaler);
  if (FAILED(status)) {
    sw_text_range_release(anchor);
    anchor = nullptr;
    throw ComFailure{status};
  }
}
TextRange::~TextRange() {
  if (marshaler)
    marshaler->Release();
  sw_text_range_release(anchor);
}
ULONG TextRange::AddRef() { return ++references; }
ULONG TextRange::Release() {
  ULONG left = --references;
  if (!left)
    delete this;
  return left;
}
HRESULT TextRange::QueryInterface(REFIID iid, void **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (iid == IID_IUnknown || iid == __uuidof(ITextRangeProvider) ||
      iid == __uuidof(ITextRangeProvider2))
    *result = static_cast<ITextRangeProvider2 *>(this);
  else if (iid == rangeIdentityId)
    *result = static_cast<RangeAccess *>(this);
  else if (iid == IID_IMarshal && marshaler)
    return marshaler->QueryInterface(iid, result);
  else
    return E_NOINTERFACE;
  AddRef();
  return S_OK;
}
HRESULT TextRange::read(TextView &view) {
  auto status = readText(element, view.state);
  if (FAILED(status))
    return status;
  std::lock_guard<std::mutex> guard(mutex);
  if (view.state->documentEpoch != epoch ||
      (view.state->document ? view.state->document->pointer : nullptr) !=
          (document ? document->pointer : nullptr))
    return UIA_E_ELEMENTNOTAVAILABLE;
  if (document) {
    auto value = sw_text_range_read(anchor, view.state->documentRevision);
    if (!value.status)
      return UIA_E_ELEMENTNOTAVAILABLE;
    view.start = value.start;
    view.end = value.end;
    view.first = value.start_affinity;
    view.last = value.end_affinity;
  } else {
    if (view.state->data.text_revision != textRevision)
      return UIA_E_ELEMENTNOTAVAILABLE;
    view.start = start;
    view.end = end;
    view.first = first;
    view.last = last;
  }
  const auto &text = *view.state->text[3];
  return view.start <= view.end && byteBoundary(text, view.start) &&
                 byteBoundary(text, view.end)
             ? S_OK
             : UIA_E_ELEMENTNOTAVAILABLE;
}
HRESULT TextRange::save(const TextView &view) {
  auto current = element->state();
  if (!current || !publicText(*current) ||
      current->data.text_revision != view.state->data.text_revision ||
      current->documentEpoch != epoch ||
      current->documentRevision != view.state->documentRevision)
    return UIA_E_ELEMENTNOTAVAILABLE;
  if (view.start > view.end || !byteBoundary(*current->text[3], view.start) ||
      !byteBoundary(*current->text[3], view.end))
    return E_INVALIDARG;
  std::lock_guard<std::mutex> guard(mutex);
  if (document)
    return sw_text_range_set(anchor, current->documentRevision, view.start,
                             view.end, view.first, view.last)
               ? S_OK
               : UIA_E_ELEMENTNOTAVAILABLE;
  if (current->data.text_revision != textRevision)
    return UIA_E_ELEMENTNOTAVAILABLE;
  start = view.start;
  end = view.end;
  first = view.first;
  last = view.start == view.end ? view.first : view.last;
  return S_OK;
}
HRESULT TextRange::ReadIdentity(RangeIdentity *result) {
  if (!result)
    return E_POINTER;
  *result = {};
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    *result = {tree->identity,
               element->serial,
               document ? view.state->documentRevision
                        : view.state->data.text_revision,
               epoch,
               document ? document->pointer : nullptr,
               view.start,
               view.end};
    return S_OK;
  });
}
HRESULT rangeIdentity(ITextRangeProvider *range, RangeIdentity &result) {
  if (!range)
    return E_INVALIDARG;
  RangeAccess *access = nullptr;
  auto status = range->QueryInterface(rangeIdentityId,
                                      reinterpret_cast<void **>(&access));
  if (FAILED(status))
    return E_INVALIDARG;
  status = access->ReadIdentity(&result);
  access->Release();
  return status;
}
bool sameDocument(const RangeIdentity &a, const RangeIdentity &b) {
  return a.tree == b.tree && a.element == b.element && a.epoch == b.epoch &&
         a.document == b.document;
}
HRESULT TextRange::Clone(ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    *result = new TextRange(element, view);
    return S_OK;
  });
}
HRESULT TextRange::Compare(ITextRangeProvider *other, BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  RangeIdentity a{}, b{};
  auto status = ReadIdentity(&a);
  if (SUCCEEDED(status))
    status = rangeIdentity(other, b);
  if (FAILED(status))
    return status;
  if (sameDocument(a, b) && a.revision != b.revision)
    return UIA_E_ELEMENTNOTAVAILABLE;
  *result = sameDocument(a, b) && a.start == b.start && a.end == b.end;
  return S_OK;
}
HRESULT TextRange::CompareEndpoints(TextPatternRangeEndpoint endpoint,
                                    ITextRangeProvider *other,
                                    TextPatternRangeEndpoint otherEndpoint,
                                    int *result) {
  if (!result)
    return E_POINTER;
  *result = 0;
  if ((endpoint != TextPatternRangeEndpoint_Start &&
       endpoint != TextPatternRangeEndpoint_End) ||
      (otherEndpoint != TextPatternRangeEndpoint_Start &&
       otherEndpoint != TextPatternRangeEndpoint_End))
    return E_INVALIDARG;
  return guarded([&]() -> HRESULT {
    RangeIdentity a{}, b{};
    auto status = ReadIdentity(&a);
    if (SUCCEEDED(status))
      status = rangeIdentity(other, b);
    if (FAILED(status))
      return status;
    if (!sameDocument(a, b))
      return E_INVALIDARG;
    if (a.revision != b.revision)
      return UIA_E_ELEMENTNOTAVAILABLE;
    TextView view;
    status = read(view);
    if (FAILED(status))
      return status;
    const auto revision = view.state->document ? view.state->documentRevision
                                               : view.state->data.text_revision;
    if (revision != a.revision)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto first = textUnits(
        *view.state->text[3],
        endpoint == TextPatternRangeEndpoint_Start ? a.start : a.end, true);
    auto last = textUnits(
        *view.state->text[3],
        otherEndpoint == TextPatternRangeEndpoint_Start ? b.start : b.end,
        true);
    if (first < 0 || last < 0)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result =
        static_cast<int>(std::clamp<int64_t>(first - last, INT_MIN, INT_MAX));
    return S_OK;
  });
}
HRESULT TextRange::GetText(int maximum, BSTR *result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (maximum < -1)
    return E_INVALIDARG;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    auto text = wide(view.state->text[3]->substr(
        static_cast<size_t>(view.start),
        static_cast<size_t>(view.end - view.start)));
    if (maximum >= 0 && text.size() > static_cast<size_t>(maximum)) {
      auto length = static_cast<size_t>(maximum);
      if (length && text[length - 1] >= 0xd800 && text[length - 1] <= 0xdbff)
        --length;
      text.resize(length);
    }
    *result = SysAllocStringLen(text.data(), static_cast<UINT>(text.size()));
    return *result ? S_OK : E_OUTOFMEMORY;
  });
}
HRESULT TextRange::GetEnclosingElement(IRawElementProviderSimple **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (SUCCEEDED(status))
      *result = simple(element);
    return status;
  });
}
HRESULT TextRange::GetChildren(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    std::vector<ElementPtr> children;
    for (const auto &child : view.state->textChildren) {
      if (child.end <= view.start || child.start >= view.end)
        continue;
      auto target = tree->actual(child.node);
      if (target && target->exposed())
        children.push_back(std::move(target));
    }
    *result = elementArray(children);
    return S_OK;
  });
}
HRESULT TextRange::Select() {
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    return actionResult(element->perform(5, {}, view.start, view.end,
                                         view.state->data.content_revision));
  });
}
HRESULT TextRange::AddToSelection() {
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    const auto &data = view.state->data;
    if (data.anchor >= 0 && data.anchor != data.cursor &&
        (std::min(data.anchor, data.cursor) != view.start ||
         std::max(data.anchor, data.cursor) != view.end))
      return UIA_E_INVALIDOPERATION;
    return actionResult(
        element->perform(5, {}, view.start, view.end, data.content_revision));
  });
}
HRESULT TextRange::RemoveFromSelection() {
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    const auto &data = view.state->data;
    if (data.anchor < 0 || std::min(data.anchor, data.cursor) != view.start ||
        std::max(data.anchor, data.cursor) != view.end)
      return UIA_E_INVALIDOPERATION;
    return actionResult(element->perform(5, {}, data.cursor, data.cursor,
                                         data.content_revision));
  });
}
HRESULT TextRange::ScrollIntoView(BOOL alignToTop) {
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    return actionResult(element->perform(21, {}, view.start, view.end,
                                         view.state->data.content_revision,
                                         alignToTop ? 3 : 11));
  });
}
HRESULT TextRange::ShowContextMenu() {
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    const auto &data = view.state->data;
    if (!(data.actions & 64))
      return UIA_E_NOTSUPPORTED;
    if (std::min(data.anchor, data.cursor) != view.start ||
        std::max(data.anchor, data.cursor) != view.end) {
      status = actionResult(
          element->perform(5, {}, view.start, view.end, data.content_revision));
      if (FAILED(status))
        return status;
    }
    return actionResult(element->perform(7));
  });
}
} // namespace swax::win
