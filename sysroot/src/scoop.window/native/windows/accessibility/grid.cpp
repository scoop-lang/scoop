#include "provider.h"

namespace swax::win {
static HRESULT integer(int64_t value, int *result) {
  if (!result)
    return E_POINTER;
  *result = 0;
  if (value < 0 || value > INT_MAX)
    return UIA_E_NOTSUPPORTED;
  *result = static_cast<int>(value);
  return S_OK;
}
HRESULT Node::GetItem(int row, int column, IRawElementProviderSimple **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (row < 0 || column < 0 || row >= element->rowCount() ||
        column >= element->columnCount())
      return E_INVALIDARG;
    auto cell = element->cellAt(row, column);
    if (!cell)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = simple(cell);
    return S_OK;
  });
}
HRESULT Node::get_RowCount(int *result) {
  return guarded([&] {
    return element->state() ? integer(element->rowCount(), result)
                            : UIA_E_ELEMENTNOTAVAILABLE;
  });
}
HRESULT Node::get_ColumnCount(int *result) {
  return guarded([&] {
    return element->state() ? integer(element->columnCount(), result)
                            : UIA_E_ELEMENTNOTAVAILABLE;
  });
}
static HRESULT cellNumber(const ElementPtr &element, int *result, int field) {
  if (!result)
    return E_POINTER;
  *result = 0;
  return guarded([&]() -> HRESULT {
    auto value = element->state();
    if (!value)
      return UIA_E_ELEMENTNOTAVAILABLE;
    return integer(field == 0   ? value->data.cell_row
                   : field == 1 ? value->data.cell_column
                   : field == 2 ? value->data.row_span
                                : value->data.column_span,
                   result);
  });
}
HRESULT Node::get_Row(int *result) { return cellNumber(element, result, 0); }
HRESULT Node::get_Column(int *result) { return cellNumber(element, result, 1); }
HRESULT Node::get_RowSpan(int *result) {
  return cellNumber(element, result, 2);
}
HRESULT Node::get_ColumnSpan(int *result) {
  return cellNumber(element, result, 3);
}
HRESULT Node::get_ContainingGrid(IRawElementProviderSimple **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto parent = element->parent();
    while (parent) {
      auto value = parent->state();
      if (!value)
        return UIA_E_ELEMENTNOTAVAILABLE;
      if (value->data.role == 22) {
        *result = simple(parent);
        return S_OK;
      }
      parent = parent->parent();
    }
    return UIA_E_NOTSUPPORTED;
  });
}
static HRESULT headers(const ElementPtr &element, bool rows,
                       SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = elementArray(element->headers(rows));
    return S_OK;
  });
}
HRESULT Node::GetRowHeaders(SAFEARRAY **result) {
  return headers(element, true, result);
}
HRESULT Node::GetColumnHeaders(SAFEARRAY **result) {
  return headers(element, false, result);
}
HRESULT Node::GetRowHeaderItems(SAFEARRAY **result) {
  return headers(element, true, result);
}
HRESULT Node::GetColumnHeaderItems(SAFEARRAY **result) {
  return headers(element, false, result);
}
HRESULT Node::get_RowOrColumnMajor(RowOrColumnMajor *result) {
  if (!result)
    return E_POINTER;
  *result = RowOrColumnMajor_RowMajor;
  return tree && !tree->closed ? S_OK : UIA_E_ELEMENTNOTAVAILABLE;
}
HRESULT Node::FindItemByProperty(IRawElementProviderSimple *after,
                                 PROPERTYID property, VARIANT value,
                                 IRawElementProviderSimple **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    auto source = element->state();
    if (!source)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (source->count < 0)
      return UIA_E_NOTSUPPORTED;
    auto previous = fromProvider(tree, after);
    if (after && (!previous || previous->reference.kind != 1 ||
                  previous->reference.node != element->reference.node))
      return E_INVALIDARG;
    Key key = previous ? previous->reference.row : Key{};
    if (property == 0 || property == UIA_ControlTypePropertyId) {
      if (property == UIA_ControlTypePropertyId) {
        if (value.vt != VT_I4)
          return E_INVALIDARG;
        if (value.lVal != UIA_ListItemControlTypeId)
          return S_OK;
      }
      int64_t index = 0;
      if (previous) {
        auto state = previous->state();
        if (!state)
          return UIA_E_ELEMENTNOTAVAILABLE;
        index = state->data.cell_row + 1;
      }
      if (index < source->count) {
        auto found = tree->itemAt(element->reference.node, index);
        if (!found)
          return UIA_E_ELEMENTNOTAVAILABLE;
        *result = simple(found);
      }
      return S_OK;
    }
    if (property != UIA_NamePropertyId &&
        property != UIA_AutomationIdPropertyId)
      return UIA_E_NOTSUPPORTED;
    if (value.vt != VT_BSTR || !value.bstrVal ||
        SysStringLen(value.bstrVal) > INT_MAX)
      return E_INVALIDARG;
    Item reply;
    reply.search =
        utf8(value.bstrVal, static_cast<int>(SysStringLen(value.bstrVal)));
    if (!reply.search)
      return E_INVALIDARG;
    auto status = tree->query(element->reference.node,
                              property == UIA_NamePropertyId ? 9 : 10,
                              previous ? 1 : 0, key, &reply);
    if (status == 2)
      return S_OK;
    if (status == 5)
      return UIA_E_NOTSUPPORTED;
    if (status != 1)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = simple(tree->item(element->reference.node, reply.key));
    return S_OK;
  });
}
} // namespace swax::win
