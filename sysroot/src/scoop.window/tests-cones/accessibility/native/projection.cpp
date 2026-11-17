#include "../../../native/accessibility/core.h"
#include <algorithm>
#include <stdexcept>

namespace {
using namespace swax;
struct Failure {
  int line;
};
#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      throw Failure{__LINE__};                                                 \
  } while (false)
class TestHost final : public Host {
public:
  bool open = true;
  bool onOwner() const override { return true; }
  int32_t dispatch(const std::function<int32_t()> &body) override {
    return body();
  }
  bool visible() const override { return open; }
  bool enabled() const override { return open; }
  bool focused() const override { return false; }
  Rect bounds() const override { return {0, 0, 640, 480}; }
  std::optional<Rect> toScreen(Rect value) const override { return value; }
  std::optional<Point> fromScreen(Point value) const override { return value; }
  void changed(const Change &) override {}
  void announce(int64_t, const std::string &, int32_t) override {}
  void close() override { open = false; }
};
std::shared_ptr<State> node(int64_t id, int64_t parent, int role) {
  auto value = std::make_shared<State>();
  value->data.number = id;
  value->data.parent = parent;
  value->data.role = role;
  value->data.flags = SWAXEnabled;
  value->data.checked = value->data.selected = value->data.expanded = -1;
  value->data.anchor = value->data.cursor = -1;
  value->data.table_rows = value->data.table_columns = -1;
  value->data.cell_row = value->data.cell_column = -1;
  return value;
}
struct Source {
  std::vector<int64_t> keys{1, 2, 3};
  int descriptions = 0;
  static int32_t query(void *context, int64_t, int32_t operation, int64_t index,
                       int32_t kind, int64_t number, const uint8_t *, int64_t,
                       void *reply) {
    auto &source = *static_cast<Source *>(context);
    if (operation == 0) {
      if (index < 0 || static_cast<size_t>(index) >= source.keys.size())
        return 2;
      number = source.keys[index];
      kind = 0;
    }
    auto at = std::find(source.keys.begin(), source.keys.end(), number);
    if (kind != 0 || at == source.keys.end())
      return 2;
    if (operation == 2)
      return 1;
    if (operation != 0 && operation != 1 && operation != 11)
      return 3;
    if (!reply)
      return 4;
    ++source.descriptions;
    auto &item = *static_cast<Item *>(reply);
    item.key = {0, number, {}};
    item.index = at - source.keys.begin();
    item.count = source.keys.size();
    item.text[0] = std::to_string(number);
    item.enabled = true;
    item.selected = number == 2 ? 1 : 0;
    return 1;
  }
};
void publish(const std::shared_ptr<Tree> &tree,
             std::initializer_list<std::shared_ptr<State>> values) {
  tree->publishing = true;
  for (const auto &value : values)
    tree->pending[value->data.number] = value;
  tree->commit(0);
}
} // namespace
extern "C" int32_t scoop_ax_test_projection_identity() {
  try {
    auto host = std::make_shared<TestHost>();
    auto tree = std::make_shared<Tree>(host, nullptr, nullptr);
    Source source;
    tree->collectionCallback = Source::query;
    tree->collectionContext = &source;
    auto list = node(1, 0, 22);
    list->count = 3;
    list->collectionRevision = 1;
    list->selectionKind = 1;
    list->selected = {{0, 2, {}}};
    list->columns = {{{1, 0, "name"}, "Name", 0},
                     {{1, 0, "value"}, "Value", 0}};
    auto mounted = node(2, 1, 0);
    mounted->itemKey = {0, 2, {}};
    mounted->data.flags |= SWAXReadOnly;
    mounted->data.geometry = mounted->data.visible = 1;
    publish(tree, {list, mounted});
    CHECK(source.descriptions == 0);
    auto row = tree->itemAt(1, 1);
    CHECK(row && row == tree->actual(2));
    CHECK(row->state()->data.flags & SWAXReadOnly);
    CHECK(row->state()->data.geometry);
    CHECK(row->indexInParent() == 1);
    auto cell = row->childAt(1);
    CHECK(cell && cell->columnIndex() == 1 && cell->parent() == row);
    CHECK(tree->actual(1)->selection().at(0) == row);
    const auto serial = row->serial;
    source.keys = {2, 3, 1};
    auto reordered = std::make_shared<State>(*list);
    reordered->collectionRevision = 2;
    publish(tree, {reordered});
    CHECK(tree->itemAt(1, 0) == row && row->serial == serial &&
          row->indexInParent() == 0);
    tree->retire(2);
    tree->removed.insert(2);
    publish(tree, {});
    CHECK(!row->retired && row->state() && !row->state()->data.geometry);
    CHECK(cell->state() && cell->state()->data.cell_row == 0);
    source.keys = {3, 1};
    auto removed = std::make_shared<State>(*reordered);
    removed->collectionRevision = 3;
    removed->count = 2;
    removed->selected.clear();
    publish(tree, {removed});
    CHECK(row->retired && !row->state() && cell->retired);
    source.keys = {2, 3, 1};
    auto reinserted = std::make_shared<State>(*removed);
    reinserted->collectionRevision = 4;
    reinserted->count = 3;
    publish(tree, {reinserted});
    auto fresh = tree->itemAt(1, 0);
    CHECK(fresh && fresh->serial != serial && !row->state());
    tree->close();
    CHECK(!fresh->state() && !cell->state() && !host->open);
    CHECK(source.descriptions < 15);
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}
extern "C" int32_t scoop_ax_test_projection_selection() {
  try {
    auto tree =
        std::make_shared<Tree>(std::make_shared<TestHost>(), nullptr, nullptr);
    auto group = node(1, 0, 0);
    group->selectionKind = 1;
    auto wrapper = node(2, 1, 0);
    wrapper->data.flags |= SWAXIgnored;
    auto selected = node(3, 2, 20);
    selected->data.selected = 1;
    auto nested = node(4, 1, 19);
    nested->selectionKind = 0;
    auto inner = node(5, 4, 20);
    inner->data.selected = 1;
    auto hidden = node(6, 1, 20);
    hidden->data.flags |= SWAXHidden;
    hidden->data.selected = 1;
    publish(tree, {group, wrapper, selected, nested, inner, hidden});
    auto outer = tree->actual(1)->selection();
    CHECK(outer.size() == 1 && outer[0] == tree->actual(3));
    CHECK(tree->actual(3)->selectionContainer() == tree->actual(1));
    CHECK(tree->actual(4)->selection().at(0) == tree->actual(5));
    CHECK(tree->actual(5)->selectionContainer() == tree->actual(4));
    tree->close();
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}
extern "C" int32_t scoop_ax_test_text_units() {
  try {
    const std::string text = "A\xF0\x9F\x98\x80"
                             "e\xCC\x81\r\nZ";
    CHECK(utf8Offset(text, 2, true) == -1);
    CHECK(utf8Offset(text, 3, true) == 5);
    CHECK(utf8Offset(text, 2, false) == 5);
    CHECK(textUnits(text, 3, true) == -1);
    CHECK(textUnits(text, text.size(), true) == 8);
    CHECK(textUnits(text, text.size(), false) == 7);
    CHECK(textSlice(text, 1, 3, true) == "\xF0\x9F\x98\x80");
    CHECK(textSlice(text, 2, 4, false) == "e\xCC\x81");
    bool rejected = false;
    try {
      textSlice(text, 1, 2, true);
    } catch (const std::invalid_argument &) {
      rejected = true;
    }
    CHECK(rejected);
    State state;
    state.data.role = 4;
    state.text[3] = text;
    CHECK(publicText(state));
    state.data.flags = SWAXProtected;
    CHECK(!publicText(state));
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}
