#include "provider.h"

namespace swax::win {
struct EventValue {
  VARIANT value{};
  EventValue() = default;
  EventValue(const EventValue &) = delete;
  EventValue(EventValue &&other) noexcept : value(other.value) {
    VariantInit(&other.value);
  }
  ~EventValue() { VariantClear(&value); }
};
static EventValue boolean(bool value) {
  EventValue result;
  result.value.vt = VT_BOOL;
  result.value.boolVal = value ? VARIANT_TRUE : VARIANT_FALSE;
  return result;
}
static EventValue integer(int32_t value) {
  EventValue result;
  result.value.vt = VT_I4;
  result.value.lVal = value;
  return result;
}
static EventValue numericValue(double value) {
  EventValue result;
  result.value.vt = VT_R8;
  result.value.dblVal = value;
  return result;
}
static EventValue string(const std::optional<std::string> &value) {
  auto text = wide(value.value_or(""));
  EventValue result;
  result.value.vt = VT_BSTR;
  result.value.bstrVal =
      SysAllocStringLen(text.data(), static_cast<UINT>(text.size()));
  if (!result.value.bstrVal)
    throw std::bad_alloc();
  return result;
}
static void property(IRawElementProviderSimple *provider, PROPERTYID property,
                     const EventValue &before, const EventValue &after) {
  UiaRaiseAutomationPropertyChangedEvent(provider, property, before.value,
                                         after.value);
}
void WindowHost::changed(const Change &change) {
  if (!UiaClientsAreListening())
    return;
  auto owner = tree.lock();
  if (!owner || owner->closed)
    return;
  try {
    if (change.structure && root)
      UiaRaiseStructureChangedEvent(
          root, StructureChangeType_ChildrenInvalidated, nullptr, 0);
    for (auto number : change.changed) {
      auto now = change.after->nodes.find(number),
           old = change.before->nodes.find(number);
      if (now == change.after->nodes.end() || old == change.before->nodes.end())
        continue;
      const State &before = *old->second, &after = *now->second;
      const auto &a = before.data, &b = after.data;
      auto element = owner->actual(number);
      auto *provider = simple(element);
      if (!provider)
        continue;
      // Keep event provider ownership local even if an allocation fails below.
      auto release = std::unique_ptr<IRawElementProviderSimple,
                                     void (*)(IRawElementProviderSimple *)>(
          provider, [](IRawElementProviderSimple *value) { value->Release(); });
      auto *node = static_cast<Node *>(provider);
      for (const auto &entry : patternAvailability) {
        const auto previous = node->supports(entry.second, before),
                   current = node->supports(entry.second, after);
        if (previous != current)
          property(provider, entry.first, boolean(previous), boolean(current));
      }
      if (controlType(before) != controlType(after))
        property(provider, UIA_ControlTypePropertyId,
                 integer(controlType(before)), integer(controlType(after)));
      if (before.text[2] != after.text[2])
        property(provider, UIA_AutomationIdPropertyId, string(before.text[2]),
                 string(after.text[2]));
      if (before.text[1] != after.text[1] || before.text[4] != after.text[4])
        property(provider, UIA_HelpTextPropertyId, string(helpText(before)),
                 string(helpText(after)));
      if (ariaProperties(before) != ariaProperties(after))
        property(provider, UIA_AriaPropertiesPropertyId,
                 string(ariaProperties(before)), string(ariaProperties(after)));
      if ((a.flags ^ b.flags) & SWAXProtected)
        property(provider, UIA_IsPasswordPropertyId,
                 boolean(a.flags & SWAXProtected),
                 boolean(b.flags & SWAXProtected));
      if (a.live != b.live)
        property(provider, UIA_LiveSettingPropertyId, integer(a.live),
                 integer(b.live));
      if (a.scroll_orientation != b.scroll_orientation)
        property(provider, UIA_OrientationPropertyId,
                 integer(a.scroll_orientation), integer(b.scroll_orientation));
      if (before.selectionKind != after.selectionKind)
        property(provider, UIA_SelectionCanSelectMultiplePropertyId,
                 boolean(before.selectionKind == 1),
                 boolean(after.selectionKind == 1));
      if (before.selectionRequired != after.selectionRequired)
        property(provider, UIA_SelectionIsSelectionRequiredPropertyId,
                 boolean(before.selectionRequired),
                 boolean(after.selectionRequired));
      if (a.role == 21 && ((a.flags ^ b.flags) & SWAXModal))
        property(provider, UIA_WindowIsModalPropertyId,
                 boolean(a.flags & SWAXModal), boolean(b.flags & SWAXModal));
      auto readOnly = [](const SWAccessibilityData &value) {
        int capability = value.has_range ? 2048 : value.role == 6 ? 128 : 4;
        return (value.flags & SWAXReadOnly) || !(value.actions & capability);
      };
      if (readOnly(a) != readOnly(b))
        property(provider,
                 b.has_range || b.role == 6 ? UIA_RangeValueIsReadOnlyPropertyId
                                            : UIA_ValueIsReadOnlyPropertyId,
                 boolean(readOnly(a)), boolean(readOnly(b)));
      if (before.text[0] != after.text[0])
        property(provider, UIA_NamePropertyId, string(before.text[0]),
                 string(after.text[0]));
      if ((a.flags ^ b.flags) & SWAXEnabled)
        property(provider, UIA_IsEnabledPropertyId,
                 boolean(a.flags & SWAXEnabled),
                 boolean(b.flags & SWAXEnabled));
      if ((a.flags ^ b.flags) & SWAXInvalid)
        property(provider, UIA_IsDataValidForFormPropertyId,
                 boolean(!(a.flags & SWAXInvalid)),
                 boolean(!(b.flags & SWAXInvalid)));
      if ((a.flags ^ b.flags) & SWAXRequired)
        property(provider, UIA_IsRequiredForFormPropertyId,
                 boolean(a.flags & SWAXRequired),
                 boolean(b.flags & SWAXRequired));
      if (a.checked != b.checked)
        property(provider, UIA_ToggleToggleStatePropertyId, integer(a.checked),
                 integer(b.checked));
      if (a.expanded != b.expanded)
        property(provider, UIA_ExpandCollapseExpandCollapseStatePropertyId,
                 integer(a.expanded < 0 ? ExpandCollapseState_LeafNode
                         : a.expanded   ? ExpandCollapseState_Expanded
                                        : ExpandCollapseState_Collapsed),
                 integer(b.expanded < 0 ? ExpandCollapseState_LeafNode
                         : b.expanded   ? ExpandCollapseState_Expanded
                                        : ExpandCollapseState_Collapsed));
      const bool selectedBefore =
          a.selected == 1 || (a.role == 9 && a.checked == 1);
      const bool selectedAfter =
          b.selected == 1 || (b.role == 9 && b.checked == 1);
      if (selectedBefore != selectedAfter) {
        property(provider, UIA_SelectionItemIsSelectedPropertyId,
                 boolean(selectedBefore), boolean(selectedAfter));
        UiaRaiseAutomationEvent(
            provider,
            selectedAfter
                ? UIA_SelectionItem_ElementSelectedEventId
                : UIA_SelectionItem_ElementRemovedFromSelectionEventId);
      }
      if (a.has_range && b.has_range && a.number_value != b.number_value)
        property(provider, UIA_RangeValueValuePropertyId,
                 numericValue(a.number_value), numericValue(b.number_value));
      if (a.has_range && b.has_range) {
        if (a.minimum != b.minimum)
          property(provider, UIA_RangeValueMinimumPropertyId,
                   numericValue(a.minimum), numericValue(b.minimum));
        if (a.maximum != b.maximum)
          property(provider, UIA_RangeValueMaximumPropertyId,
                   numericValue(a.maximum), numericValue(b.maximum));
        if (a.number_step != b.number_step)
          property(provider, UIA_RangeValueSmallChangePropertyId,
                   numericValue(a.number_step), numericValue(b.number_step));
      }
      if (b.role == 6 && a.scroll_offset != b.scroll_offset)
        property(provider, UIA_RangeValueValuePropertyId,
                 numericValue(a.scroll_offset), numericValue(b.scroll_offset));
      if (!(b.flags & SWAXProtected) && before.text[3] != after.text[3]) {
        if (b.role == 4)
          property(
              provider, UIA_ValueValuePropertyId,
              string(a.flags & SWAXProtected ? std::nullopt : before.text[3]),
              string(after.text[3]));
        if (publicText(after))
          UiaRaiseAutomationEvent(provider, UIA_Text_TextChangedEventId);
      }
      if (publicText(after) && (a.anchor != b.anchor || a.cursor != b.cursor))
        UiaRaiseAutomationEvent(provider, UIA_Text_TextSelectionChangedEventId);
      if (before.selected != after.selected)
        UiaRaiseAutomationEvent(provider, UIA_Selection_InvalidatedEventId);
      if (before.collectionRevision != after.collectionRevision)
        UiaRaiseStructureChangedEvent(
            provider, StructureChangeType_ChildrenInvalidated, nullptr, 0);
      if (before.relations != after.relations ||
          before.activeCollection != after.activeCollection ||
          before.activeKey != after.activeKey)
        UiaRaiseAutomationEvent(provider, UIA_LayoutInvalidatedEventId);
      if (a.geometry != b.geometry || a.visible != b.visible || a.x != b.x ||
          a.y != b.y || a.width != b.width || a.height != b.height)
        UiaRaiseAutomationEvent(provider, UIA_LayoutInvalidatedEventId);
    }
    if (change.before->focused != change.after->focused &&
        change.after->focused && focused()) {
      if (auto provider = simple(owner->actual(change.after->focused))) {
        UiaRaiseAutomationEvent(provider, UIA_AutomationFocusChangedEventId);
        provider->Release();
      }
    }
  } catch (const ComFailure &error) {
    if (window)
      sw_fail(window->application, error.value);
  } catch (const std::bad_alloc &) {
    if (window)
      sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
  } catch (...) {
    if (window)
      sw_fail(window->application, ERROR_INVALID_DATA);
  }
}
void WindowHost::announce(int64_t number, const std::string &message,
                          int32_t priority) {
  auto owner = tree.lock();
  if (!owner || owner->closed || !UiaClientsAreListening())
    return;
  try {
    auto text = string(message);
    auto activity =
        string(std::to_string(owner->identity) + ":" + std::to_string(number));
    auto element = owner->actual(number);
    if (!element || !element->exposed())
      return;
    auto provider = simple(element);
    if (!provider)
      return;
    UiaRaiseNotificationEvent(provider, NotificationKind_Other,
                              priority == 2
                                  ? NotificationProcessing_ImportantMostRecent
                                  : NotificationProcessing_MostRecent,
                              text.value.bstrVal, activity.value.bstrVal);
    provider->Release();
  } catch (const ComFailure &error) {
    if (window)
      sw_fail(window->application, error.value);
  } catch (const std::bad_alloc &) {
    if (window)
      sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
  } catch (...) {
    if (window)
      sw_fail(window->application, ERROR_INVALID_DATA);
  }
}
void WindowHost::nativeChanged(UINT message) {
  auto owner = tree.lock();
  if (!owner || owner->closed || !root || !UiaClientsAreListening())
    return;
  if (message == WM_MOVE || message == WM_SIZE || message == WM_DPICHANGED ||
      message == WM_SHOWWINDOW)
    UiaRaiseAutomationEvent(root, UIA_LayoutInvalidatedEventId);
  if (message != WM_SETFOCUS && message != WM_KILLFOCUS)
    return;
  try {
    const auto focusedNode = owner->snapshot()->focused;
    auto provider = simple(owner->actual(focusedNode));
    if (!provider)
      return;
    property(provider, UIA_HasKeyboardFocusPropertyId,
             boolean(message == WM_KILLFOCUS), boolean(message == WM_SETFOCUS));
    if (message == WM_SETFOCUS && focusedNode)
      UiaRaiseAutomationEvent(provider, UIA_AutomationFocusChangedEventId);
    provider->Release();
  } catch (const ComFailure &error) {
    if (window)
      sw_fail(window->application, error.value);
  } catch (const std::bad_alloc &) {
    if (window)
      sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
  } catch (...) {
    if (window)
      sw_fail(window->application, ERROR_INVALID_DATA);
  }
}
} // namespace swax::win
