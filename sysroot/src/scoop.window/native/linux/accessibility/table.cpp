#include "provider.h"
#include <algorithm>

namespace swax::atspi {
ElementPtr containingTable(const ElementPtr &element) {
  auto parent = element->parent();
  while (parent) {
    auto state = parent->state();
    if (!state)
      return {};
    if ((state->data.role == 22 || state->data.role == 29))
      return parent;
    parent = parent->parent();
  }
  return {};
}
std::vector<int> selectedRows(const ElementPtr &element) {
  std::set<int> rows;
  for (const auto &item : element->selection()) {
    auto state = item->state();
    if (!state)
      unavailable();
    if (state->data.cell_row >= 0)
      rows.insert(integer(state->data.cell_row));
  }
  return {rows.begin(), rows.end()};
}
static GVariant *integers(const std::vector<int> &values) {
  GVariantBuilder result;
  g_variant_builder_init(&result, G_VARIANT_TYPE("ai"));
  for (int value : values)
    g_variant_builder_add(&result, "i", value);
  return g_variant_builder_end(&result);
}
static ElementPtr row(const ElementPtr &table, int64_t index) {
  if (index < 0 || index >= table->rowCount())
    invalid("Table row is out of range");
  auto state = table->state();
  if (!state)
    unavailable();
  if (state->count >= 0)
    return table->childAt(index);
  // Static tables only traverse their mounted row hierarchy.
  std::function<ElementPtr(const ElementPtr &)> find =
      [&](const ElementPtr &parent) -> ElementPtr {
    for (int64_t i = 0, count = parent->childCount(); i < count; ++i) {
      auto child = parent->childAt(i);
      auto value = child ? child->state() : nullptr;
      if (!value)
        continue;
      if ((value->data.role == 28 || value->data.role == 30) &&
          value->data.cell_row == index)
        return child;
      if ((value->data.role != 22 && value->data.role != 29) &&
          (value->data.role == 0 ||
           (value->data.role == 28 || value->data.role == 30)))
        if (auto result = find(child))
          return result;
    }
    return {};
  };
  return find(table);
}
static bool rowSelected(const ElementPtr &element, int index) {
  if (index < 0 || index >= element->rowCount())
    invalid("Table row is out of range");
  auto rows = selectedRows(element);
  return std::find(rows.begin(), rows.end(), index) != rows.end();
}
static bool columnSelected(const ElementPtr &element, int index) {
  if (index < 0 || index >= element->columnCount())
    invalid("Table column is out of range");
  auto count = element->rowCount();
  return count > 0 &&
         static_cast<int64_t>(selectedRows(element).size()) == count;
}
static ElementPtr tableCell(const ElementPtr &element, int row, int column) {
  if (row < 0 || column < 0 || row >= element->rowCount() ||
      column >= element->columnCount())
    invalid("Table cell is out of range");
  auto cell = element->cellAt(row, column);
  if (!cell)
    unavailable();
  return cell;
}
GVariant *table(Request &request, const char *interface, const char *method,
                GVariant *parameters) {
  const auto &data = request.state->data;
  auto element = request.element;
  if (!strcmp(interface, "org.a11y.atspi.TableCell")) {
    if (!strcmp(method, "GetRowColumnSpan"))
      return g_variant_new("(biiii)", TRUE, integer(data.cell_row),
                           integer(data.cell_column), integer(data.row_span),
                           integer(data.column_span));
    if (!strcmp(method, "GetColumnHeaderCells") ||
        !strcmp(method, "GetRowHeaderCells"))
      return g_variant_new(
          "(@a(so))",
          request.host->references(
              element->headers(!strcmp(method, "GetRowHeaderCells")),
              request.sender));
    unsupported();
  }
  const auto rows = element->rowCount(), columns = element->columnCount();
  if (!strcmp(method, "GetSelectedRows"))
    return g_variant_new("(@ai)", integers(selectedRows(element)));
  if (!strcmp(method, "GetSelectedColumns")) {
    std::vector<int> selected;
    if (rows > 0 && static_cast<int64_t>(selectedRows(element).size()) == rows)
      for (int64_t column = 0; column < columns; ++column)
        selected.push_back(integer(column));
    return g_variant_new("(@ai)", integers(selected));
  }
  if (!strcmp(method, "GetAccessibleAt") || !strcmp(method, "GetIndexAt") ||
      !strcmp(method, "GetRowExtentAt") ||
      !strcmp(method, "GetColumnExtentAt") || !strcmp(method, "IsSelected")) {
    int r, c;
    g_variant_get(parameters, "(ii)", &r, &c);
    auto cell = tableCell(element, r, c);
    auto state = cell->state();
    if (!state)
      unavailable();
    if (!strcmp(method, "GetAccessibleAt"))
      return g_variant_new("(@(so))",
                           request.host->reference(cell, request.sender));
    if (!strcmp(method, "GetIndexAt")) {
      if (columns > INT_MAX || static_cast<int64_t>(r) * columns > INT_MAX - c)
        unsupported("Table index exceeds AT-SPI range");
      return g_variant_new("(i)", static_cast<int>(r * columns + c));
    }
    if (!strcmp(method, "GetRowExtentAt"))
      return g_variant_new("(i)", integer(state->data.row_span));
    if (!strcmp(method, "GetColumnExtentAt"))
      return g_variant_new("(i)", integer(state->data.column_span));
    return g_variant_new("(b)", state->data.selected >= 0
                                    ? selected(*state)
                                    : rowSelected(element, r));
  }
  int index;
  g_variant_get(parameters, "(i)", &index);
  if (!strcmp(method, "GetRowAtIndex") || !strcmp(method, "GetColumnAtIndex") ||
      !strcmp(method, "GetRowColumnExtentsAtIndex")) {
    if (index < 0 || columns <= 0 || index / columns >= rows)
      invalid("Table index is out of range");
    const auto r = index / columns, c = index % columns;
    if (!strcmp(method, "GetRowAtIndex"))
      return g_variant_new("(i)", integer(r));
    if (!strcmp(method, "GetColumnAtIndex"))
      return g_variant_new("(i)", integer(c));
    auto cell = tableCell(element, integer(r), integer(c));
    auto state = cell->state();
    if (!state)
      unavailable();
    return g_variant_new(
        "(biiiib)", TRUE, integer(r), integer(c), integer(state->data.row_span),
        integer(state->data.column_span),
        state->data.selected >= 0 ? selected(*state)
                                  : rowSelected(element, integer(r)));
  }
  if (!strcmp(method, "IsRowSelected"))
    return g_variant_new("(b)", rowSelected(element, index));
  if (!strcmp(method, "IsColumnSelected"))
    return g_variant_new("(b)", columnSelected(element, index));
  if (!strcmp(method, "AddRowSelection") ||
      !strcmp(method, "RemoveRowSelection"))
    return g_variant_new(
        "(b)", select(row(element, index), !strcmp(method, "AddRowSelection")));
  if (!strcmp(method, "AddColumnSelection") ||
      !strcmp(method, "RemoveColumnSelection"))
    unsupported(
        "This table provides row selection, not a column selection model");
  if (!strcmp(method, "GetRowHeader") || !strcmp(method, "GetColumnHeader") ||
      !strcmp(method, "GetRowDescription") ||
      !strcmp(method, "GetColumnDescription")) {
    bool isRow =
        !strcmp(method, "GetRowHeader") || !strcmp(method, "GetRowDescription");
    if (index < 0 || index >= (isRow ? rows : columns))
      invalid("Table header index is out of range");
    ElementPtr header;
    auto owner = element->tree.lock();
    if (!isRow && request.state->count >= 0 &&
        static_cast<size_t>(index) < request.state->columns.size()) {
      const auto &column = request.state->columns[index];
      header = column.header
                   ? owner->actual(column.header)
                   : owner->column(element->reference.node, column.key);
    } else {
      auto cell = rows && columns
                      ? element->cellAt(isRow ? index : 0, isRow ? 0 : index)
                      : nullptr;
      if (cell) {
        const auto state = cell->state();
        // A header cell has no relation to itself.
        if (state && state->data.role == (isRow ? 24 : 25))
          header = cell;
        else {
          auto headers = cell->headers(isRow);
          if (!headers.empty())
            header = headers.front();
        }
      }
    }
    if (!strcmp(method, "GetRowDescription") ||
        !strcmp(method, "GetColumnDescription")) {
      auto state = header ? header->state() : nullptr;
      return g_variant_new("(@s)",
                           string(state ? state->text[0].value_or("") : ""));
    }
    return g_variant_new("(@(so))",
                         request.host->reference(header, request.sender));
  }
  unsupported();
}
} // namespace swax::atspi
