#include "provider.h"
#include <algorithm>
#include <cmath>

namespace swax::win {
// UIA specifies -1 for an axis that is absent or should remain unchanged.
static constexpr double noScroll = -1;

ElementPtr Node::scrollBar(int orientation) {
  if (!tree || element->retired)
    return {};
  auto current = tree->snapshot();
  auto node = tree->state(element->reference.node);
  if (!current || !node)
    return {};
  if ((node->data.role == 12 || node->data.role == 22) && node->data.parent) {
    auto parent = tree->state(node->data.parent);
    if (parent && parent->data.role == 5)
      node = parent;
  }
  auto children = current->children.find(node->data.number);
  if (children == current->children.end())
    return {};
  for (auto number : children->second) {
    auto child = tree->state(number);
    if (child && child->data.role == 6 &&
        child->data.scroll_orientation == orientation &&
        !(child->data.flags & (SWAXHidden | SWAXIgnored)))
      return tree->actual(number);
  }
  return {};
}
static HRESULT scrollMetric(Node &node, int orientation, int field,
                            double *result) {
  if (!result)
    return E_POINTER;
  *result = field ? 100 : noScroll;
  return guarded([&]() -> HRESULT {
    if (!node.element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto bar = node.scrollBar(orientation);
    auto value = bar ? bar->state() : nullptr;
    if (!value)
      return S_OK;
    double maximum =
        std::max(0.0, value->data.scroll_content - value->data.scroll_viewport);
    if (field)
      *result = value->data.scroll_content > 0
                    ? std::min(100.0, value->data.scroll_viewport /
                                          value->data.scroll_content * 100)
                    : 100;
    else if (maximum > 0)
      *result = value->data.scroll_offset / maximum * 100;
    return S_OK;
  });
}
HRESULT Node::get_HorizontalScrollPercent(double *result) {
  return scrollMetric(*this, 1, 0, result);
}
HRESULT Node::get_VerticalScrollPercent(double *result) {
  return scrollMetric(*this, 2, 0, result);
}
HRESULT Node::get_HorizontalViewSize(double *result) {
  return scrollMetric(*this, 1, 1, result);
}
HRESULT Node::get_VerticalViewSize(double *result) {
  return scrollMetric(*this, 2, 1, result);
}
HRESULT Node::get_HorizontallyScrollable(BOOL *result) {
  if (!result)
    return E_POINTER;
  double percent;
  HRESULT status = get_HorizontalScrollPercent(&percent);
  *result = SUCCEEDED(status) && percent != noScroll;
  return status;
}
HRESULT Node::get_VerticallyScrollable(BOOL *result) {
  if (!result)
    return E_POINTER;
  double percent;
  HRESULT status = get_VerticalScrollPercent(&percent);
  *result = SUCCEEDED(status) && percent != noScroll;
  return status;
}
HRESULT Node::SetScrollPercent(double horizontal, double vertical) {
  return guarded([&]() -> HRESULT {
    if (!std::isfinite(horizontal) || !std::isfinite(vertical) ||
        (horizontal != noScroll && (horizontal < 0 || horizontal > 100)) ||
        (vertical != noScroll && (vertical < 0 || vertical > 100)))
      return E_INVALIDARG;
    auto x = scrollBar(1), y = scrollBar(2);
    if ((horizontal != noScroll && !x) || (vertical != noScroll && !y))
      return UIA_E_INVALIDOPERATION;
    if (horizontal != noScroll) {
      auto status =
          actionResult(x->perform(8, {}, -1, -1, 0, horizontal / 100));
      if (FAILED(status))
        return status;
    }
    return vertical == noScroll
               ? S_OK
               : actionResult(y->perform(8, {}, -1, -1, 0, vertical / 100));
  });
}
static bool validAmount(ScrollAmount value) {
  return value == ScrollAmount_NoAmount ||
         value == ScrollAmount_LargeDecrement ||
         value == ScrollAmount_LargeIncrement ||
         value == ScrollAmount_SmallDecrement ||
         value == ScrollAmount_SmallIncrement;
}
static HRESULT scroll(const ElementPtr &bar, ScrollAmount amount) {
  if (amount == ScrollAmount_NoAmount)
    return S_OK;
  if (!bar)
    return UIA_E_INVALIDOPERATION;
  if (amount == ScrollAmount_SmallDecrement ||
      amount == ScrollAmount_SmallIncrement)
    return actionResult(
        bar->perform(amount == ScrollAmount_SmallDecrement ? 9 : 10));
  auto value = bar->state();
  if (!value)
    return UIA_E_ELEMENTNOTAVAILABLE;
  double maximum =
      std::max(0.0, value->data.scroll_content - value->data.scroll_viewport);
  if (maximum == 0)
    return UIA_E_INVALIDOPERATION;
  double next = value->data.scroll_offset +
                (amount == ScrollAmount_LargeDecrement ? -1 : 1) *
                    value->data.scroll_viewport;
  return actionResult(
      bar->perform(8, {}, -1, -1, 0, std::clamp(next / maximum, 0.0, 1.0)));
}
HRESULT Node::Scroll(ScrollAmount horizontal, ScrollAmount vertical) {
  return guarded([&]() -> HRESULT {
    if (!validAmount(horizontal) || !validAmount(vertical))
      return E_INVALIDARG;
    auto x = scrollBar(1), y = scrollBar(2);
    if ((horizontal != ScrollAmount_NoAmount && !x) ||
        (vertical != ScrollAmount_NoAmount && !y))
      return UIA_E_INVALIDOPERATION;
    auto status = scroll(x, horizontal);
    return FAILED(status) ? status : scroll(y, vertical);
  });
}
} // namespace swax::win
