#include "text.h"
#include <algorithm>
#include <exception>

namespace swax::win {
HRESULT TextRange::GetAttributeValue(TEXTATTRIBUTEID attribute,
                                     VARIANT *result) {
  if (!result)
    return E_POINTER;
  VariantInit(result);
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    if (attribute == UIA_IsReadOnlyAttributeId) {
      result->vt = VT_BOOL;
      result->boolVal = (view.state->data.flags & SWAXReadOnly) ||
                                !(view.state->data.actions & (4 | 32))
                            ? VARIANT_TRUE
                            : VARIANT_FALSE;
      return S_OK;
    }
    result->vt = VT_UNKNOWN;
    return UiaGetReservedNotSupportedValue(&result->punkVal);
  });
}
HRESULT TextRange::FindAttribute(TEXTATTRIBUTEID attribute, VARIANT value, BOOL,
                                 ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (attribute != UIA_IsReadOnlyAttributeId)
    return UIA_E_NOTSUPPORTED;
  if (value.vt != VT_BOOL)
    return E_INVALIDARG;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    bool readOnly = (view.state->data.flags & SWAXReadOnly) ||
                    !(view.state->data.actions & (4 | 32));
    if (readOnly == (value.boolVal != VARIANT_FALSE))
      *result = new TextRange(element, view);
    return S_OK;
  });
}
HRESULT TextRange::FindText(BSTR text, BOOL backward, BOOL ignoreCase,
                            ITextRangeProvider **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (!text || !SysStringLen(text) || SysStringLen(text) > INT_MAX)
    return E_INVALIDARG;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    auto content = wide(view.state->text[3]->substr(
        static_cast<size_t>(view.start),
        static_cast<size_t>(view.end - view.start)));
    const auto length = static_cast<int>(SysStringLen(text));
    if (!utf8(text, length))
      return E_INVALIDARG;
    auto found = FindStringOrdinal(
        backward ? FIND_FROMEND : FIND_FROMSTART, content.data(),
        static_cast<int>(content.size()), text, length, ignoreCase);
    if (found < 0)
      return S_OK;
    const auto &source = *view.state->text[3];
    const auto base = textUnits(source, view.start, true);
    view.start = utf8Offset(source, base + found, true);
    view.end = utf8Offset(source, base + found + length, true);
    if (view.start < 0 || view.end < 0)
      return E_INVALIDARG;
    view.first = 1;
    view.last = 0;
    *result = new TextRange(element, view);
    return S_OK;
  });
}
struct TextBoxes {
  std::vector<Rect> values;
  std::exception_ptr failure;
};
static void collectBox(void *context, double x, double y, double width,
                       double height) noexcept {
  auto &boxes = *static_cast<TextBoxes *>(context);
  if (boxes.failure)
    return;
  try {
    boxes.values.push_back({x, y, width, height});
  } catch (...) {
    boxes.failure = std::current_exception();
  }
}
HRESULT TextRange::GetBoundingRectangles(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    TextView view;
    auto status = read(view);
    if (FAILED(status))
      return status;
    if (!view.state->textGeometry)
      return UIA_E_ELEMENTNOTAVAILABLE;
    const auto &layout = view.state->textGeometry->data;
    TextBoxes boxes;
    auto measured =
        layout.boxes(layout.context, view.start, view.end, &boxes, collectBox);
    if (boxes.failure)
      std::rethrow_exception(boxes.failure);
    status = textGeometryResult(measured);
    if (FAILED(status))
      return status;
    std::vector<double> values;
    for (const auto &box : boxes.values) {
      auto screen = tree->host->toScreen(box);
      if (!screen)
        return UIA_E_ELEMENTNOTAVAILABLE;
      values.insert(values.end(),
                    {screen->x, screen->y, screen->width, screen->height});
    }
    *result = numberArray(values);
    return S_OK;
  });
}
} // namespace swax::win
