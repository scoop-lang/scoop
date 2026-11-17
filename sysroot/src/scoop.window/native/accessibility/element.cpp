#include "core.h"
#include <algorithm>

namespace swax {
std::shared_ptr<const State> Element::state() {
  auto owner = tree.lock();
  if (!owner || retired)
    return {};
  auto source = owner->state(reference.node);
  if (!source || !reference.kind)
    return source;
  if (source->count < 0)
    return {};
  const auto column = columnIndex();
  if (reference.kind >= 2 && column < 0)
    return {};
  if (reference.kind == 3) {
    const auto &definition = source->columns[column];
    if (definition.header)
      return owner->state(definition.header);
    auto result = std::make_shared<State>();
    result->data.flags = SWAXEnabled;
    result->data.role = 25;
    result->data.checked = result->data.selected = result->data.expanded = -1;
    result->data.cell_row = -1;
    result->data.cell_column = column;
    result->data.row_span = result->data.column_span = 1;
    result->text[0] = definition.name;
    return result;
  }
  std::optional<Item> data;
  {
    std::lock_guard<std::mutex> lock(mutex);
    if (cached && cachedRevision == source->collectionRevision)
      data = cached;
  }
  if (!data) {
    Item value;
    const auto status =
        owner->query(reference.node, reference.kind == 2 ? 11 : 1,
                     reference.kind == 2 ? column : 0, reference.row, &value);
    if (status == 2)
      retired = true;
    if (status != 1)
      return {};
    auto current = owner->state(reference.node);
    if (!current || current->collectionRevision != source->collectionRevision)
      return {};
    data = value;
    std::lock_guard<std::mutex> lock(mutex);
    cached = std::move(value);
    cachedRevision = source->collectionRevision;
  }
  auto mounted = owner->mounted(reference.node, reference.row,
                                reference.kind == 2 ? reference.column : Key{});
  auto result =
      mounted ? std::make_shared<State>(*mounted) : std::make_shared<State>();
  result->data.role = reference.kind == 2 ? 23 : 28;
  result->data.flags =
      (result->data.flags & ~SWAXEnabled) |
      (data->enabled && (!mounted || (mounted->data.flags & SWAXEnabled))
           ? SWAXEnabled
           : 0);
  result->data.actions = 0;
  result->data.anchor = result->data.cursor = -1;
  result->data.checked = -1;
  result->data.selected = data->selected;
  result->data.expanded = data->expanded;
  result->data.parent = reference.node;
  result->data.cell_row = data->index;
  result->data.cell_column = reference.kind == 2 ? column : -1;
  result->data.row_span = result->data.column_span = 1;
  result->data.table_rows = result->data.table_columns = -1;
  if (reference.kind == 1) {
    if ((source->collectionActions & 4) && data->focusable)
      result->data.actions |= 8;
    if (source->collectionActions & 2)
      result->data.actions |= 1024;
    if ((source->collectionActions & 8) && source->selectionKind >= 0)
      result->data.actions |= 16384;
  }
  for (size_t index = 0; index < data->text.size(); ++index)
    result->text[index] = data->text[index];
  if (reference.kind == 2 && source->columns[column].header)
    result->relations[6] = {source->columns[column].header};
  return result;
}
bool Element::exposed(bool enabled) {
  auto owner = tree.lock();
  auto value = state();
  return owner && value && !(value->data.flags & (SWAXHidden | SWAXIgnored)) &&
         (!enabled || (value->data.flags & SWAXEnabled)) &&
         owner->exposed(reference.node, enabled);
}
ElementPtr Element::parent() {
  auto owner = tree.lock();
  if (!owner || retired)
    return {};
  if (reference.kind == 2)
    return owner->item(reference.node, reference.row);
  if (reference.kind)
    return owner->actual(reference.node);
  auto value = owner->state(reference.node);
  if (!value || !reference.node)
    return {};
  auto parent = owner->state(value->data.parent);
  while (parent && parent->data.number && (parent->data.flags & SWAXIgnored) &&
         !parent->itemKey)
    parent = owner->state(parent->data.parent);
  return parent ? owner->actual(parent->data.number) : nullptr;
}
std::vector<ElementPtr> Element::actualChildren() {
  std::vector<ElementPtr> result;
  auto owner = tree.lock();
  if (!owner || retired)
    return result;
  int64_t number = reference.node;
  if (reference.kind == 3)
    return result;
  if (reference.kind) {
    auto mounted =
        owner->mounted(reference.node, reference.row,
                       reference.kind == 2 ? reference.column : Key{});
    if (!mounted)
      return result;
    number = mounted->data.number;
  }
  auto current = owner->snapshot();
  if (!current)
    return result;
  std::function<void(int64_t)> append = [&](int64_t parent) {
    auto found = current->children.find(parent);
    if (found == current->children.end())
      return;
    for (auto child : found->second) {
      auto value = owner->state(child);
      if (!value || (value->data.flags & SWAXHidden))
        continue;
      if ((value->data.flags & SWAXIgnored) && !value->itemKey)
        append(child);
      else if (auto element = owner->actual(child))
        result.push_back(element);
    }
  };
  append(number);
  return result;
}
int64_t Element::childCount() {
  auto owner = tree.lock();
  auto value = state();
  if (!owner || !value)
    return 0;
  auto source = owner->state(reference.node);
  if (reference.kind == 0 && source->count >= 0)
    return source->count;
  if (reference.kind == 1 && !source->columns.empty())
    return static_cast<int64_t>(source->columns.size());
  return static_cast<int64_t>(actualChildren().size());
}
ElementPtr Element::childAt(int64_t index) {
  if (index < 0)
    return {};
  auto owner = tree.lock();
  auto value = state();
  if (!owner || !value)
    return {};
  auto source = owner->state(reference.node);
  if (reference.kind == 0 && source->count >= 0)
    return index < source->count ? owner->itemAt(reference.node, index)
                                 : nullptr;
  if (reference.kind == 1 && !source->columns.empty())
    return static_cast<uint64_t>(index) < source->columns.size()
               ? owner->cell(reference.node, reference.row,
                             source->columns[index].key)
               : nullptr;
  auto children = actualChildren();
  return static_cast<uint64_t>(index) < children.size() ? children[index]
                                                        : nullptr;
}
int64_t Element::columnIndex() const {
  auto owner = tree.lock();
  auto source = owner ? owner->state(reference.node) : nullptr;
  if (!source)
    return -1;
  for (size_t index = 0; index < source->columns.size(); ++index)
    if (source->columns[index].key == reference.column)
      return static_cast<int64_t>(index);
  return -1;
}
int64_t Element::indexInParent() {
  if (reference.kind == 1) {
    auto value = state();
    return value ? value->data.cell_row : -1;
  }
  if (reference.kind >= 2)
    return columnIndex();
  auto container = parent();
  if (!container)
    return -1;
  auto children = container->actualChildren();
  for (size_t index = 0; index < children.size(); ++index)
    if (children[index]->reference == reference)
      return static_cast<int64_t>(index);
  return -1;
}
ElementPtr Element::selectionContainer() {
  auto current = parent();
  while (current) {
    auto value = current->state();
    if (!value)
      return {};
    if (value->selectionKind >= 0)
      return current;
    current = current->parent();
  }
  return {};
}
std::vector<ElementPtr> Element::selection() {
  std::vector<ElementPtr> result;
  auto owner = tree.lock();
  auto value = state();
  if (!owner || !value)
    return result;
  if (value->selectionKind >= 0 && value->count >= 0) {
    for (const auto &key : value->selected)
      if (auto selected = owner->item(reference.node, key))
        result.push_back(selected);
    return result;
  }
  // This traversal contains mounted nodes only, never logical collection items.
  std::function<void(const ElementPtr &)> append =
      [&](const ElementPtr &parent) {
        for (const auto &child : parent->actualChildren()) {
          auto data = child->state();
          if (!data)
            continue;
          if (data->data.selected == 1 ||
              (data->data.role == 9 && data->data.checked == 1))
            result.push_back(child);
          else if (data->selectionKind < 0 && data->count < 0)
            append(child);
        }
      };
  append(shared_from_this());
  return result;
}
int32_t Element::perform(int32_t action, const std::optional<std::string> &text,
                         int64_t start, int64_t end, uint64_t revision,
                         double position) {
  auto owner = tree.lock();
  if (!owner || retired)
    return 4;
  if (!reference.kind)
    return owner->perform(reference.node, action, text, start, end, revision,
                          position);
  if (reference.kind != 1)
    return 3;
  if (action == 3 || action == 4)
    return owner->operate(reference.node, reference.row, action == 3 ? 5 : 6);
  if (action == 11)
    return owner->operate(reference.node, reference.row, 4);
  if (action == 15 || action == 16)
    return owner->operate(reference.node, reference.row, action == 15 ? 7 : 8);
  return 3;
}
int32_t Element::realize() {
  auto owner = tree.lock();
  if (!owner || retired)
    return 4;
  return reference.kind == 1 || reference.kind == 2
             ? owner->operate(reference.node, reference.row, 3)
             : 3;
}
} // namespace swax
