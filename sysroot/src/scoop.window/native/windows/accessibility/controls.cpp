#include "provider.h"
#include <algorithm>
#include <cmath>

namespace swax::win {
HRESULT Node::Invoke() {
  return guarded([&] { return actionResult(element->perform(0)); });
}
HRESULT Node::Toggle() {
  return guarded([&] { return actionResult(element->perform(1)); });
}
HRESULT Node::get_ToggleState(ToggleState *result) {
  if (!result)
    return E_POINTER;
  *result = ToggleState_Off;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (state->data.checked < 0)
      return UIA_E_NOTSUPPORTED;
    *result = state->data.checked == 2 ? ToggleState_Indeterminate
              : state->data.checked    ? ToggleState_On
                                       : ToggleState_Off;
    return S_OK;
  });
}
HRESULT Node::SetValue(LPCWSTR value) {
  return guarded([&]() -> HRESULT {
    auto text = utf8(value);
    return text ? actionResult(element->perform(2, text)) : E_INVALIDARG;
  });
}
HRESULT Node::get_Value(BSTR *result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (state->data.flags & SWAXProtected)
      return E_ACCESSDENIED;
    if (!state->text[3])
      return UIA_E_NOTSUPPORTED;
    auto text = wide(*state->text[3]);
    *result = SysAllocStringLen(text.data(), static_cast<UINT>(text.size()));
    return *result ? S_OK : E_OUTOFMEMORY;
  });
}
HRESULT Node::get_IsReadOnly(BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = TRUE;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    int capability = state->data.has_range   ? 2048
                     : state->data.role == 6 ? 128
                                             : 4;
    *result = (state->data.flags & SWAXReadOnly) ||
              !(state->data.actions & capability);
    return S_OK;
  });
}
HRESULT Node::SetValue(double value) {
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (!std::isfinite(value))
      return E_INVALIDARG;
    if (state->data.has_range)
      return actionResult(element->perform(12, {}, -1, -1, 0, value));
    if (state->data.role != 6)
      return UIA_E_NOTSUPPORTED;
    const double maximum =
        std::max(0.0, state->data.scroll_content - state->data.scroll_viewport);
    if (value < 0 || value > maximum)
      return E_INVALIDARG;
    return actionResult(
        element->perform(8, {}, -1, -1, 0, maximum > 0 ? value / maximum : 0));
  });
}
static HRESULT numeric(const ElementPtr &element, double *result, int field) {
  if (!result)
    return E_POINTER;
  *result = 0;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    const auto &d = state->data;
    if (!d.has_range && d.role != 6)
      return UIA_E_NOTSUPPORTED;
    switch (field) {
    case 0:
      *result = d.has_range ? d.number_value : d.scroll_offset;
      break;
    case 1:
      *result = d.has_range ? d.minimum : 0;
      break;
    case 2:
      *result = d.has_range
                    ? d.maximum
                    : std::max(0.0, d.scroll_content - d.scroll_viewport);
      break;
    case 3:
      *result = d.has_range ? d.number_step : d.scroll_viewport;
      break;
    case 4:
      if (!d.has_range && !d.number_step)
        return UIA_E_NOTSUPPORTED;
      *result = d.number_step;
      break;
    }
    return S_OK;
  });
}
HRESULT Node::get_Value(double *result) { return numeric(element, result, 0); }
HRESULT Node::get_Minimum(double *result) {
  return numeric(element, result, 1);
}
HRESULT Node::get_Maximum(double *result) {
  return numeric(element, result, 2);
}
HRESULT Node::get_LargeChange(double *result) {
  return numeric(element, result, 3);
}
HRESULT Node::get_SmallChange(double *result) {
  return numeric(element, result, 4);
}
HRESULT Node::GetSelection(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = elementArray(element->selection());
    return S_OK;
  });
}
HRESULT Node::get_CanSelectMultiple(BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = state->selectionKind == 1;
    return S_OK;
  });
}
HRESULT Node::get_IsSelectionRequired(BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = state->selectionRequired;
    return S_OK;
  });
}
HRESULT Node::get_IsSelected(BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (state->data.selected < 0 && state->data.role != 9)
      return UIA_E_NOTSUPPORTED;
    *result = state->data.selected == 1 ||
              (state->data.role == 9 && state->data.checked == 1);
    return S_OK;
  });
}
HRESULT Node::get_SelectionContainer(IRawElementProviderSimple **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = simple(element->selectionContainer());
    return S_OK;
  });
}
HRESULT Node::select(bool value, bool exclusive) {
  auto state = element->state();
  if (!state)
    return UIA_E_ELEMENTNOTAVAILABLE;
  const bool selected = state->data.selected == 1 ||
                        (state->data.role == 9 && state->data.checked == 1);
  auto container = element->selectionContainer();
  auto source = container ? container->state() : nullptr;
  if (!value && selected && source && source->selectionRequired &&
      container->selection().size() <= 1)
    return UIA_E_INVALIDOPERATION;
  if (value && exclusive && source && source->selectionKind == 1) {
    if (!selected) {
      auto status = actionResult(element->perform(15));
      if (FAILED(status))
        return status;
    }
    for (const auto &other : container->selection()) {
      if (other->reference == element->reference)
        continue;
      auto status = actionResult(other->perform(16));
      if (FAILED(status))
        return status;
    }
    return S_OK;
  } else if (value && !exclusive && source && source->selectionKind != 1 &&
             !selected && !container->selection().empty())
    return UIA_E_INVALIDOPERATION;
  if (selected == value)
    return S_OK;
  if (state->data.role == 9)
    return value ? actionResult(element->perform(0)) : UIA_E_INVALIDOPERATION;
  return actionResult(element->perform(value ? 15 : 16));
}
HRESULT Node::Select() {
  return guarded([&] { return select(true, true); });
}
HRESULT Node::AddToSelection() {
  return guarded([&] { return select(true, false); });
}
HRESULT Node::RemoveFromSelection() {
  return guarded([&] { return select(false, false); });
}
HRESULT Node::Expand() {
  return guarded([&] { return actionResult(element->perform(17)); });
}
HRESULT Node::Collapse() {
  return guarded([&] { return actionResult(element->perform(18)); });
}
HRESULT Node::get_ExpandCollapseState(ExpandCollapseState *result) {
  if (!result)
    return E_POINTER;
  *result = ExpandCollapseState_LeafNode;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (state->data.expanded >= 0)
      *result = state->data.expanded ? ExpandCollapseState_Expanded
                                     : ExpandCollapseState_Collapsed;
    return S_OK;
  });
}
HRESULT Node::ScrollIntoView() {
  return guarded([&] { return actionResult(element->perform(11)); });
}
HRESULT Node::Realize() {
  return guarded([&] { return actionResult(element->realize()); });
}
} // namespace swax::win
