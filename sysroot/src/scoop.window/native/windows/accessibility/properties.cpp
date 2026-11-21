#include "provider.h"

namespace swax::win {
CONTROLTYPEID controlType(const State &value) {
  const auto &data = value.data;
  if (data.role == 4 && data.expanded >= 0)
    return UIA_ComboBoxControlTypeId;
  switch (data.role) {
  case 1:
  case 14:
    return UIA_TextControlTypeId;
  case 2:
  case 8:
    return UIA_ButtonControlTypeId;
  case 3:
  case 7:
    return UIA_CheckBoxControlTypeId;
  case 4:
    return UIA_EditControlTypeId;
  case 5:
    return UIA_PaneControlTypeId;
  case 6:
    return UIA_ScrollBarControlTypeId;
  case 9:
    return UIA_RadioButtonControlTypeId;
  case 11:
    return UIA_ImageControlTypeId;
  case 12:
    return UIA_ListControlTypeId;
  case 13:
    return UIA_HyperlinkControlTypeId;
  case 15:
    return UIA_SliderControlTypeId;
  case 16:
    return UIA_ProgressBarControlTypeId;
  case 17:
    return UIA_MenuControlTypeId;
  case 18:
    return UIA_MenuItemControlTypeId;
  case 19:
    return UIA_TabControlTypeId;
  case 20:
    return UIA_TabItemControlTypeId;
  case 21:
    return UIA_WindowControlTypeId;
  case 22:
    return UIA_TableControlTypeId;
  case 23:
    return UIA_DataItemControlTypeId;
  case 24:
  case 25:
    return UIA_HeaderItemControlTypeId;
  case 26:
    return UIA_ButtonControlTypeId;
  case 27:
    return data.has_range ? UIA_ThumbControlTypeId : UIA_SeparatorControlTypeId;
  case 28:
    return UIA_ListItemControlTypeId;
  case 29:
    return UIA_TreeControlTypeId;
  case 30:
    return UIA_TreeItemControlTypeId;
  default:
    return UIA_GroupControlTypeId;
  }
}
std::string helpText(const State &value) {
  auto text = value.text[1].value_or("");
  if (value.text[4]) {
    if (!text.empty())
      text += "\n";
    text += *value.text[4];
  }
  return text;
}
std::string ariaProperties(const State &value) {
  const auto &data = value.data;
  std::string properties =
      (data.flags & SWAXBusy) ? "busy=true;" : "busy=false;";
  if (data.flags & SWAXInvalid)
    properties += "invalid=true;";
  if (data.flags & SWAXModal)
    properties += "modal=true;";
  if (data.expanded >= 0)
    properties += data.expanded ? "expanded=true;" : "expanded=false;";
  return properties;
}
bool Node::supports(PATTERNID pattern, const State &value) {
  const auto &d = value.data;
  switch (pattern) {
  case UIA_WindowPatternId:
    return d.role == 21;
  case UIA_InvokePatternId:
    return (d.actions & 1) != 0;
  case UIA_TogglePatternId:
    return d.checked >= 0 && (d.role == 3 || d.role == 7 || d.role == 8 ||
                              d.role == 18 || d.role == 30);
  case UIA_ValuePatternId:
    return d.role == 4 || ((d.actions & 4) != 0);
  case UIA_RangeValuePatternId:
    return d.has_range || d.role == 6;
  case UIA_SelectionPatternId:
    return value.selectionKind >= 0;
  case UIA_SelectionItemPatternId:
    return d.selected >= 0 || d.role == 9;
  case UIA_ExpandCollapsePatternId:
    return d.expanded >= 0;
  case UIA_ScrollPatternId:
    return (d.role == 5 || d.role == 4 || d.role == 12 || d.role == 22 ||
            d.role == 29) &&
           (scrollBar(1) || scrollBar(2));
  case UIA_ScrollItemPatternId:
    return (d.actions & 1024) != 0;
  case UIA_GridPatternId:
  case UIA_TablePatternId:
    return d.role == 22 ||
           (d.role == 29 && (!value.columns.empty() || d.table_columns > 0));
  case UIA_GridItemPatternId:
  case UIA_TableItemPatternId:
    return d.cell_row >= 0 && d.cell_column >= 0 && d.row_span > 0 &&
           d.column_span > 0;
  case UIA_ItemContainerPatternId:
    return value.count >= 0;
  case UIA_VirtualizedItemPatternId: {
    auto source = tree ? tree->state(element->reference.node) : nullptr;
    return (element->reference.kind == 1 || element->reference.kind == 2) &&
           source && (source->collectionActions & 1);
  }
  case UIA_TextPatternId:
  case UIA_TextPattern2Id:
    return publicText(value);
  default:
    return false;
  }
}
HRESULT Node::GetPatternProvider(PATTERNID pattern, IUnknown **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    auto value = element->state();
    if (!value)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (!supports(pattern, *value))
      return S_OK;
    switch (pattern) {
    case UIA_WindowPatternId:
      *result = static_cast<IWindowProvider *>(this);
      break;
    case UIA_InvokePatternId:
      *result = static_cast<IInvokeProvider *>(this);
      break;
    case UIA_TogglePatternId:
      *result = static_cast<IToggleProvider *>(this);
      break;
    case UIA_ValuePatternId:
      *result = static_cast<IValueProvider *>(this);
      break;
    case UIA_RangeValuePatternId:
      *result = static_cast<IRangeValueProvider *>(this);
      break;
    case UIA_SelectionPatternId:
      *result = static_cast<ISelectionProvider *>(this);
      break;
    case UIA_SelectionItemPatternId:
      *result = static_cast<ISelectionItemProvider *>(this);
      break;
    case UIA_ExpandCollapsePatternId:
      *result = static_cast<IExpandCollapseProvider *>(this);
      break;
    case UIA_ScrollPatternId:
      *result = static_cast<IScrollProvider *>(this);
      break;
    case UIA_ScrollItemPatternId:
      *result = static_cast<IScrollItemProvider *>(this);
      break;
    case UIA_GridPatternId:
      *result = static_cast<IGridProvider *>(this);
      break;
    case UIA_GridItemPatternId:
      *result = static_cast<IGridItemProvider *>(this);
      break;
    case UIA_TablePatternId:
      *result = static_cast<ITableProvider *>(this);
      break;
    case UIA_TableItemPatternId:
      *result = static_cast<ITableItemProvider *>(this);
      break;
    case UIA_ItemContainerPatternId:
      *result = static_cast<IItemContainerProvider *>(this);
      break;
    case UIA_VirtualizedItemPatternId:
      *result = static_cast<IVirtualizedItemProvider *>(this);
      break;
    case UIA_TextPatternId:
    case UIA_TextPattern2Id:
      *result = textProvider(element);
      return S_OK;
    default:
      return S_OK;
    }
    AddRef();
    return S_OK;
  });
}
static void stringValue(VARIANT *result, const std::string &text) {
  auto value = wide(text);
  auto string =
      SysAllocStringLen(value.data(), static_cast<UINT>(value.size()));
  if (!string)
    throw std::bad_alloc();
  result->vt = VT_BSTR;
  result->bstrVal = string;
}
HRESULT Node::GetPropertyValue(PROPERTYID property, VARIANT *result) {
  if (!result)
    return E_POINTER;
  VariantInit(result);
  return guarded([&]() -> HRESULT {
    auto value = element->state();
    if (!value || !tree)
      return UIA_E_ELEMENTNOTAVAILABLE;
    const auto &data = value->data;
    auto boolean = [&](bool flag) {
      result->vt = VT_BOOL;
      result->boolVal = flag ? VARIANT_TRUE : VARIANT_FALSE;
    };
    auto integer = [&](int value) {
      result->vt = VT_I4;
      result->lVal = value;
    };
    auto related = [&](size_t field) {
      std::vector<ElementPtr> nodes;
      for (auto number : value->relations[field])
        if (auto node = tree->actual(number))
          nodes.push_back(node);
      result->parray = elementArray(nodes);
      result->vt = VT_ARRAY | VT_UNKNOWN;
    };
    for (const auto &entry : patternAvailability)
      if (entry.first == property) {
        boolean(supports(entry.second, *value));
        return S_OK;
      }
    switch (property) {
    case UIA_ControlTypePropertyId:
      integer(!element->reference.node && !element->reference.kind
                  ? UIA_PaneControlTypeId
                  : controlType(*value));
      break;
    case UIA_NamePropertyId:
      stringValue(result, value->text[0].value_or(""));
      break;
    case UIA_AutomationIdPropertyId:
      stringValue(result, value->text[2].value_or(""));
      break;
    case UIA_HelpTextPropertyId:
      stringValue(result, helpText(*value));
      break;
    case UIA_FrameworkIdPropertyId:
      stringValue(result, "Scoop");
      break;
    case UIA_ProcessIdPropertyId:
      integer(static_cast<int>(GetCurrentProcessId()));
      break;
    case UIA_NativeWindowHandlePropertyId:
      integer(!element->reference.node && !element->reference.kind
                  ? static_cast<int>(reinterpret_cast<INT_PTR>(
                        std::static_pointer_cast<WindowHost>(tree->host)->hwnd))
                  : 0);
      break;
    case UIA_IsControlElementPropertyId:
    case UIA_IsContentElementPropertyId:
      boolean(element->exposed());
      break;
    case UIA_IsEnabledPropertyId:
      boolean(element->exposed(true));
      break;
    case UIA_IsKeyboardFocusablePropertyId:
      boolean((data.actions & 8) != 0 && element->exposed(true));
      break;
    case UIA_HasKeyboardFocusPropertyId: {
      auto current = tree->snapshot();
      auto focused = current && current->focused
                         ? tree->actual(current->focused)
                         : nullptr;
      boolean(focused && focused->reference == element->reference &&
              tree->host->focused());
      break;
    }
    case UIA_IsOffscreenPropertyId:
      boolean(!element->exposed() ||
              ((element->reference.node || element->reference.kind) &&
               (!data.geometry || !data.visible || data.visible_width <= 0 ||
                data.visible_height <= 0)));
      break;
    case UIA_IsPasswordPropertyId:
      boolean((data.flags & SWAXProtected) != 0);
      break;
    case UIA_IsRequiredForFormPropertyId:
      boolean((data.flags & SWAXRequired) != 0);
      break;
    case UIA_IsDataValidForFormPropertyId:
      boolean(!(data.flags & SWAXInvalid));
      break;
    case UIA_IsDialogPropertyId:
      boolean(data.role == 21);
      break;
    case UIA_LiveSettingPropertyId:
      integer(data.live);
      break;
    case UIA_OrientationPropertyId:
      integer(data.scroll_orientation == 1   ? OrientationType_Horizontal
              : data.scroll_orientation == 2 ? OrientationType_Vertical
                                             : OrientationType_None);
      break;
    case UIA_LevelPropertyId:
      if (value->hierarchyLevel >= 0) {
        if (value->hierarchyLevel < INT_MAX)
          integer(static_cast<int>(value->hierarchyLevel + 1));
      } else if (data.heading_level)
        integer(data.heading_level);
      break;
    case UIA_HeadingLevelPropertyId:
      if (data.heading_level > 0 && data.heading_level <= 6)
        // UIA defines HeadingLevel1 as 80051; MinGW omits these identifiers.
        integer(80050 + data.heading_level);
      break;
    case UIA_AriaPropertiesPropertyId:
      stringValue(result, ariaProperties(*value));
      break;
    case UIA_ControllerForPropertyId:
      related(1);
      break;
    case UIA_DescribedByPropertyId:
      related(3);
      break;
    case UIA_LabeledByPropertyId:
      if (!(data.flags & SWAXNamed) && value->relations[0].size() == 1) {
        result->punkVal = simple(tree->actual(value->relations[0][0]));
        result->vt = VT_UNKNOWN;
      }
      break;
    case UIA_BoundingRectanglePropertyId: {
      UiaRect rect{};
      HRESULT status = get_BoundingRectangle(&rect);
      if (FAILED(status))
        return status;
      result->parray =
          numberArray({rect.left, rect.top, rect.width, rect.height});
      result->vt = VT_ARRAY | VT_R8;
      break;
    }
    case UIA_ClickablePointPropertyId:
      if (data.activation && element->exposed(true)) {
        auto rect =
            tree->host->toScreen({data.activation_x, data.activation_y, 0, 0});
        if (rect) {
          result->parray = numberArray({rect->x, rect->y});
          result->vt = VT_ARRAY | VT_R8;
        }
      }
      break;
    case UIA_PositionInSetPropertyId: {
      const auto index =
          value->siblingIndex >= 0 ? value->siblingIndex : data.cell_row;
      if (element->reference.kind == 1 && index >= 0 && index < INT_MAX)
        integer(static_cast<int>(index + 1));
      break;
    }
    case UIA_SizeOfSetPropertyId: {
      auto source = tree->state(element->reference.node);
      const auto count = value->siblingCount >= 0 ? value->siblingCount
                         : source                 ? source->count
                                                  : -1;
      if (element->reference.kind == 1 && count >= 0 && count <= INT_MAX)
        integer(static_cast<int>(count));
      break;
    }
    default:
      break;
    }
    return S_OK;
  });
}
} // namespace swax::win
