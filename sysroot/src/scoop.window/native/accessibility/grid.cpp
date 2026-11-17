#include "core.h"

namespace swax {
int64_t Element::rowCount() {
  auto value = state();
  if (!value || value->data.role != 22)
    return 0;
  return value->count >= 0 ? value->count : value->data.table_rows;
}
int64_t Element::columnCount() {
  auto value = state();
  if (!value || value->data.role != 22)
    return 0;
  return value->count >= 0 ? static_cast<int64_t>(value->columns.size())
                           : value->data.table_columns;
}
ElementPtr Element::cellAt(int64_t row, int64_t column) {
  auto owner = tree.lock();
  auto value = state();
  if (!owner || !value || value->data.role != 22 || row < 0 || column < 0 ||
      row >= rowCount() || column >= columnCount())
    return {};
  if (value->count >= 0) {
    auto item = owner->itemAt(reference.node, row);
    return item ? owner->cell(reference.node, item->reference.row,
                              value->columns[column].key)
                : nullptr;
  }
  std::vector<ElementPtr> search = actualChildren();
  while (!search.empty()) {
    auto child = search.back();
    search.pop_back();
    auto data = child->state();
    if (!data)
      continue;
    const auto &d = data->data;
    if (d.cell_row >= 0 && d.cell_column >= 0 && row >= d.cell_row &&
        column >= d.cell_column && row - d.cell_row < d.row_span &&
        column - d.cell_column < d.column_span)
      return child;
    if (d.role != 22) {
      auto nested = child->actualChildren();
      search.insert(search.end(), nested.begin(), nested.end());
    }
  }
  return {};
}
std::vector<ElementPtr> Element::headers(bool rows) {
  std::vector<ElementPtr> result;
  auto owner = tree.lock();
  auto value = state();
  if (!owner || !value)
    return result;
  if (value->data.role != 22) {
    for (auto number : value->relations[rows ? 5 : 6])
      if (auto target = owner->actual(number))
        result.push_back(target);
    if (result.empty() && !rows && reference.kind == 2)
      if (auto target = owner->column(reference.node, reference.column))
        result.push_back(target);
    return result;
  }
  if (value->count >= 0) {
    if (!rows)
      for (const auto &column : value->columns) {
        auto target = column.header ? owner->actual(column.header)
                                    : owner->column(reference.node, column.key);
        if (target)
          result.push_back(target);
      }
    return result;
  }
  std::function<void(const ElementPtr &)> append =
      [&](const ElementPtr &parent) {
        for (const auto &child : parent->actualChildren()) {
          auto data = child->state();
          if (!data)
            continue;
          if (data->data.role == (rows ? 24 : 25))
            result.push_back(child);
          else if (data->data.role != 22)
            append(child);
        }
      };
  append(shared_from_this());
  return result;
}
} // namespace swax
