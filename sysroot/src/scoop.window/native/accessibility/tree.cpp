#include "core.h"
#include <algorithm>
#include <limits>
#include <stdexcept>
#include <tuple>

namespace swax {
static std::atomic<uint64_t> nextTree{0};

bool Key::operator<(const Key &other) const {
  return std::tie(kind, number, text) <
         std::tie(other.kind, other.number, other.text);
}
bool Key::operator==(const Key &other) const {
  return kind == other.kind && number == other.number && text == other.text;
}
bool Reference::operator<(const Reference &other) const {
  return std::tie(kind, node, row, column) <
         std::tie(other.kind, other.node, other.row, other.column);
}
bool Reference::operator==(const Reference &other) const {
  return kind == other.kind && node == other.node && row == other.row &&
         column == other.column;
}

Tree::Tree(std::shared_ptr<Host> value, SWAccessibilityCallback action,
           void *argument)
    : host(std::move(value)), identity(++nextTree), callback(action),
      context(argument) {
  auto initial = std::make_shared<Snapshot>();
  auto root = std::make_shared<State>();
  root->data.flags = SWAXEnabled;
  root->data.checked = root->data.selected = root->data.expanded = -1;
  root->data.anchor = root->data.cursor = -1;
  root->data.table_rows = root->data.table_columns = -1;
  root->data.cell_row = root->data.cell_column = -1;
  initial->nodes[0] = root;
  published = initial;
}
std::shared_ptr<const Snapshot> Tree::snapshot() const {
  std::lock_guard<std::mutex> lock(mutex);
  return closed ? nullptr : published;
}
std::shared_ptr<const State> Tree::state(int64_t number) const {
  std::lock_guard<std::mutex> lock(mutex);
  if (closed || retiring.count(number))
    return {};
  auto found = published->nodes.find(number);
  return found == published->nodes.end() ? nullptr : found->second;
}
ElementPtr Tree::element(Reference ref) {
  if (!state(ref.node))
    return {};
  std::lock_guard<std::mutex> lock(mutex);
  auto &entry = elements[ref];
  if (auto existing = entry.lock())
    if (!existing->retired)
      return existing;
  auto result = std::make_shared<Element>(weak_from_this(), std::move(ref),
                                          ++nextElement);
  entry = result;
  return result;
}
ElementPtr Tree::actual(int64_t number) {
  auto value = state(number);
  if (!value)
    return {};
  if (value->itemKey)
    return item(value->data.parent, value->itemKey);
  if (value->cellKey) {
    auto parent = state(value->data.parent);
    while (parent) {
      if (parent->itemKey)
        return cell(parent->data.parent, parent->itemKey, value->cellKey);
      if (!parent->data.number)
        break;
      parent = state(parent->data.parent);
    }
  }
  return element({0, number, {}, {}});
}
ElementPtr Tree::elementBySerial(uint64_t serial) {
  std::lock_guard<std::mutex> lock(mutex);
  if (closed)
    return {};
  for (const auto &entry : elements)
    if (auto value = entry.second.lock())
      if (!value->retired && value->serial == serial)
        return value;
  return {};
}
ElementPtr Tree::itemAt(int64_t owner, int64_t index) {
  Item value;
  return query(owner, 0, index, {}, &value) == 1 ? item(owner, value.key)
                                                 : nullptr;
}
ElementPtr Tree::item(int64_t owner, const Key &key) {
  return key ? element({1, owner, key, {}}) : nullptr;
}
ElementPtr Tree::cell(int64_t owner, const Key &row, const Key &column) {
  return row && column ? element({2, owner, row, column}) : nullptr;
}
ElementPtr Tree::column(int64_t owner, const Key &key) {
  return key ? element({3, owner, {}, key}) : nullptr;
}
std::shared_ptr<const State> Tree::mounted(int64_t owner, const Key &row,
                                           const Key &column) const {
  auto current = snapshot();
  if (!current)
    return {};
  auto direct = current->children.find(owner);
  if (direct == current->children.end())
    return {};
  for (auto number : direct->second) {
    auto value = current->nodes.at(number);
    if (value->itemKey != row)
      continue;
    if (!column)
      return state(number);
    std::vector<int64_t> search{number};
    while (!search.empty()) {
      auto candidate = current->nodes.at(search.back());
      search.pop_back();
      if (candidate->cellKey == column)
        return state(candidate->data.number);
      auto children = current->children.find(candidate->data.number);
      if (children != current->children.end())
        search.insert(search.end(), children->second.begin(),
                      children->second.end());
    }
    break;
  }
  return {};
}
bool Tree::exposed(int64_t number, bool requireEnabled) const {
  if (closed || !host->visible() || (requireEnabled && !host->enabled()))
    return false;
  auto current = state(number);
  while (current) {
    if ((current->data.flags & SWAXHidden) ||
        (requireEnabled && !(current->data.flags & SWAXEnabled)))
      return false;
    if (!current->data.number)
      return true;
    current = state(current->data.parent);
  }
  return false;
}
ElementPtr Tree::hit(Point point, int64_t ancestor) {
  auto current = snapshot();
  if (!current || !exposed(ancestor))
    return {};
  for (auto number : current->hitOrder) {
    auto value = state(number);
    if (!value || !value->hit || !exposed(number) ||
        !value->hit->data.contains(value->hit->data.context, point.x, point.y))
      continue;
    bool inside = ancestor == 0;
    auto parent = value;
    while (parent && !inside) {
      inside = parent->data.number == ancestor;
      if (!parent->data.number)
        break;
      parent = state(parent->data.parent);
    }
    if (!inside)
      continue;
    while (value && (value->data.flags & SWAXIgnored) && !value->itemKey)
      value = state(value->data.parent);
    if (value)
      return actual(value->data.number);
  }
  return ancestor == 0 ? actual(0) : nullptr;
}
int32_t Tree::query(int64_t owner, int32_t operation, int64_t index,
                    const Key &key, Item *reply) {
  auto self = shared_from_this();
  return host->dispatch([=] {
    auto source = self->state(owner);
    if (self->closed || !source || source->count < 0 ||
        !self->collectionCallback)
      return 4;
    if (self->publishing && operation != 2)
      return operation >= 3 && operation <= 8 ? 6 : 4;
    const auto result = self->collectionCallback(
        self->collectionContext, owner, operation, index,
        key.kind < 0 ? 0 : key.kind, key.number,
        key.kind == 1 ? reinterpret_cast<const uint8_t *>(key.text.data())
                      : nullptr,
        static_cast<int64_t>(key.text.size()), reply);
    return reply && reply->failed ? 4 : result;
  });
}
int32_t actionCapability(int32_t action) {
  static const int32_t bits[] = {
      1,    2,    4,    8,    8,     16,    32,    64,    128,   256, 512,
      1024, 2048, 4096, 8192, 16384, 16384, 32768, 32768, 65536, 0,   131072};
  return action >= 0 && action < 22 ? bits[action] : 0;
}
int32_t Tree::perform(int64_t number, int32_t action,
                      const std::optional<std::string> &text, int64_t start,
                      int64_t end, uint64_t revision, double position) {
  auto self = shared_from_this();
  return host->dispatch([=] {
    auto value = self->state(number);
    if (self->closed || !value || !self->callback)
      return 4;
    if (self->publishing)
      return 6;
    if (!self->exposed(number, true))
      return 5;
    if (!(value->data.actions & actionCapability(action)))
      return 3;
    return self->callback(self->context, number, action,
                          text ? reinterpret_cast<const uint8_t *>(text->data())
                               : nullptr,
                          text ? static_cast<int64_t>(text->size()) : 0, start,
                          end, revision, position);
  });
}
int32_t Tree::operate(int64_t owner, const Key &key, int32_t operation) {
  if (!exposed(owner, true))
    return 5;
  return query(owner, operation, 0, key, nullptr);
}

void Tree::refreshProxies(const std::vector<ElementPtr> &proxies,
                          const std::shared_ptr<const Snapshot> &next) {
  for (const auto &proxy : proxies) {
    const auto &ref = proxy->reference;
    if (removed.count(ref.node)) {
      proxy->retired = true;
      continue;
    }
    if (!ref.kind || !pending.count(ref.node))
      continue;
    auto source = next->nodes.find(ref.node);
    if (source == next->nodes.end() || source->second->count < 0) {
      proxy->retired = true;
      continue;
    }
    if (ref.kind != 3 && query(ref.node, 2, 0, ref.row, nullptr) == 2)
      proxy->retired = true;
    if (ref.kind >= 2) {
      bool present = false;
      for (const auto &column : source->second->columns)
        if (column.key == ref.column) {
          present = true;
          break;
        }
      if (!present)
        proxy->retired = true;
    }
  }
}
void Tree::commit(int64_t focused) {
  if (closed || !publishing || !host->onOwner())
    throw std::logic_error("invalid accessibility publication");
  auto before = snapshot();
  auto next = std::make_shared<Snapshot>(*before);
  Change change;
  change.before = before;
  change.after = next;
  bool hitChanged = false;
  for (auto number : removed) {
    next->nodes.erase(number);
    change.removed.push_back(number);
    change.structure = hitChanged = true;
  }
  for (const auto &entry : pending) {
    auto previous = before->nodes.find(entry.first);
    const auto &value = entry.second;
    if (previous == before->nodes.end())
      change.structure = hitChanged = true;
    else {
      const auto &old = previous->second;
      change.structure |=
          old->data.parent != value->data.parent ||
          old->data.order != value->data.order ||
          ((old->data.flags ^ value->data.flags) & (SWAXHidden | SWAXIgnored));
      hitChanged |= old->data.scene_order != value->data.scene_order ||
                    (old->hit ? old->hit->data.context : nullptr) !=
                        (value->hit ? value->hit->data.context : nullptr);
    }
    next->nodes[entry.first] = value;
    change.changed.push_back(entry.first);
  }
  next->focused = focused;
  if (change.structure) {
    next->children.clear();
    for (const auto &entry : next->nodes)
      if (entry.first)
        next->children[entry.second->data.parent].push_back(entry.first);
    for (auto &entry : next->children)
      std::sort(entry.second.begin(), entry.second.end(),
                [&next](int64_t a, int64_t b) {
                  const auto &left = next->nodes.at(a)->data,
                             &right = next->nodes.at(b)->data;
                  return std::tie(left.order, left.number) <
                         std::tie(right.order, right.number);
                });
  }
  if (hitChanged) {
    next->hitOrder.clear();
    for (const auto &entry : next->nodes)
      if (entry.second->hit)
        next->hitOrder.push_back(entry.first);
    std::sort(next->hitOrder.begin(), next->hitOrder.end(),
              [&next](int64_t a, int64_t b) {
                const auto &left = next->nodes.at(a)->data,
                           &right = next->nodes.at(b)->data;
                return std::tie(left.scene_order, left.number) >
                       std::tie(right.scene_order, right.number);
              });
  }
  std::vector<ElementPtr> proxies;
  {
    std::lock_guard<std::mutex> lock(mutex);
    published = next;
    for (auto at = elements.begin(); at != elements.end();) {
      if (auto proxy = at->second.lock()) {
        proxies.push_back(proxy);
        ++at;
      } else
        at = elements.erase(at);
    }
  }
  refreshProxies(proxies, next);
  {
    std::lock_guard<std::mutex> lock(mutex);
    for (auto number : removed)
      retiring.erase(number);
  }
  pending.clear();
  removed.clear();
  publishing = false;
  host->changed(change);
}
void Tree::retire(int64_t number) {
  std::lock_guard<std::mutex> lock(mutex);
  retiring.insert(number);
  for (const auto &entry : elements)
    if (entry.first.node == number)
      if (auto proxy = entry.second.lock())
        proxy->retired = true;
}
void Tree::close() {
  if (closed.exchange(true))
    return;
  callback = nullptr;
  collectionCallback = nullptr;
  context = collectionContext = nullptr;
  host->close();
  std::lock_guard<std::mutex> lock(mutex);
  for (const auto &entry : elements)
    if (auto proxy = entry.second.lock())
      proxy->retired = true;
  elements.clear();
  pending.clear();
  removed.clear();
  published.reset();
}
} // namespace swax
