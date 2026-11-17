#include "text.h"
#include <algorithm>
#include <cstdint>

namespace swax::win {
HRESULT enclosingUnit(const TextView &view, TextUnit unit, int64_t offset,
                      int32_t affinity, SWAccessibilityTextRange &result) {
  if (unit < TextUnit_Character || unit > TextUnit_Document)
    return E_INVALIDARG;
  if (unit == TextUnit_Document || unit == TextUnit_Page) {
    result = {0, static_cast<int64_t>(view.state->text[3]->size()), 1};
    return S_OK;
  }
  if (!view.state->textGeometry)
    return UIA_E_ELEMENTNOTAVAILABLE;
  const auto &layout = view.state->textGeometry->data;
  int32_t nativeUnit = unit == TextUnit_Character   ? 0
                       : unit == TextUnit_Line      ? 2
                       : unit == TextUnit_Paragraph ? 3
                                                    : 1;
  result = layout.range(layout.context, offset, affinity, nativeUnit);
  return result.status == 1   ? S_OK
         : result.status == 0 ? UIA_E_ELEMENTNOTAVAILABLE
                              : textGeometryResult(result.status);
}
static HRESULT moveOffset(const TextView &view, TextUnit unit, int count,
                          int64_t &offset, int &moved) {
  moved = 0;
  if (unit < TextUnit_Character || unit > TextUnit_Document)
    return E_INVALIDARG;
  const int direction = count < 0 ? -1 : 1;
  const int64_t requested = count < 0 ? -static_cast<int64_t>(count) : count;
  for (int64_t index = 0; index < requested; ++index) {
    SWAccessibilityTextRange unitRange{};
    auto status =
        enclosingUnit(view, unit, offset, direction > 0 ? 1 : 0, unitRange);
    if (FAILED(status))
      return status;
    const auto next = direction > 0 ? unitRange.end : unitRange.start;
    if (next == offset)
      break;
    offset = next;
    moved += direction;
  }
  return S_OK;
}
HRESULT TextRange::ExpandToEnclosingUnit(TextUnit unit) {
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    SWAccessibilityTextRange range{};
    auto affinity =
        view.start == static_cast<int64_t>(view.state->text[3]->size())
            ? 0
            : view.first;
    status = enclosingUnit(view, unit, view.start, affinity, range);
    if (FAILED(status))
      return status;
    view.start = range.start;
    view.end = range.end;
    view.first = 1;
    view.last = 0;
    return save(view);
  });
}
HRESULT TextRange::Move(TextUnit unit, int count, int *result) {
  if (!result)
    return E_POINTER;
  *result = 0;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    if (unit < TextUnit_Character || unit > TextUnit_Document)
      return E_INVALIDARG;
    if (!count)
      return S_OK;
    const bool caret = view.start == view.end;
    if (!caret) {
      SWAccessibilityTextRange initial{};
      status = enclosingUnit(view, unit, view.start, view.first, initial);
      if (FAILED(status))
        return status;
      view.start = initial.start;
      view.end = initial.end;
    }
    auto offset = view.start;
    int moved = 0;
    status = moveOffset(view, unit, count, offset, moved);
    if (FAILED(status))
      return status;
    if (caret)
      view.start = view.end = offset;
    else {
      // A nondegenerate range must remain on a complete unit at the end of the
      // document.
      if (moved > 0 &&
          offset == static_cast<int64_t>(view.state->text[3]->size())) {
        SWAccessibilityTextRange previous{};
        status = enclosingUnit(view, unit, offset, 0, previous);
        if (FAILED(status))
          return status;
        offset = previous.start;
        --moved;
      }
      SWAccessibilityTextRange target{};
      status = enclosingUnit(view, unit, offset, 1, target);
      if (FAILED(status))
        return status;
      view.start = target.start;
      view.end = target.end;
    }
    view.first = 1;
    view.last = caret ? 1 : 0;
    status = save(view);
    if (SUCCEEDED(status))
      *result = moved;
    return status;
  });
}
HRESULT TextRange::MoveEndpointByUnit(TextPatternRangeEndpoint endpoint,
                                      TextUnit unit, int count, int *result) {
  if (!result)
    return E_POINTER;
  *result = 0;
  if (endpoint != TextPatternRangeEndpoint_Start &&
      endpoint != TextPatternRangeEndpoint_End)
    return E_INVALIDARG;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    int moved = 0;
    auto offset =
        endpoint == TextPatternRangeEndpoint_Start ? view.start : view.end;
    status = moveOffset(view, unit, count, offset, moved);
    if (FAILED(status))
      return status;
    if (endpoint == TextPatternRangeEndpoint_Start) {
      view.start = offset;
      view.end = std::max(view.end, offset);
      view.first = 1;
    } else {
      view.end = offset;
      view.start = std::min(view.start, offset);
      view.last = 0;
    }
    status = save(view);
    if (SUCCEEDED(status))
      *result = moved;
    return status;
  });
}
HRESULT TextRange::MoveEndpointByRange(TextPatternRangeEndpoint endpoint,
                                       ITextRangeProvider *other,
                                       TextPatternRangeEndpoint otherEndpoint) {
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
    if ((view.state->document ? view.state->documentRevision
                              : view.state->data.text_revision) != a.revision)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto offset =
        otherEndpoint == TextPatternRangeEndpoint_Start ? b.start : b.end;
    if (endpoint == TextPatternRangeEndpoint_Start) {
      view.start = offset;
      view.end = std::max(view.end, offset);
      view.first = 1;
    } else {
      view.end = offset;
      view.start = std::min(view.start, offset);
      view.last = 0;
    }
    return save(view);
  });
}
} // namespace swax::win
