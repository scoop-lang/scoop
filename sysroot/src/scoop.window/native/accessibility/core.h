#ifndef SCOOP_WINDOW_ACCESSIBILITY_CORE_H
#define SCOOP_WINDOW_ACCESSIBILITY_CORE_H

#include "../accessibility_abi.h"
#include "../accessibility_text_document.h"
#include <array>
#include <atomic>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <string>
#include <vector>

namespace swax {

struct Key {
  int32_t kind = -1;
  int64_t number = 0;
  std::string text;
  bool operator<(const Key &other) const;
  bool operator==(const Key &other) const;
  bool operator!=(const Key &other) const { return !(*this == other); }
  explicit operator bool() const { return kind >= 0; }
};

struct Reference {
  // Actual node, logical row, logical cell, or a metadata column header.
  int32_t kind = 0;
  int64_t node = 0;
  Key row, column;
  bool operator<(const Reference &other) const;
  bool operator==(const Reference &other) const;
};

struct Hit {
  SWAccessibilityHitData data;
  explicit Hit(SWAccessibilityHitData value) : data(value) {
    data.retain(data.context);
  }
  ~Hit() { data.release(data.context); }
};
struct TextGeometry {
  SWAccessibilityTextData data;
  explicit TextGeometry(SWAccessibilityTextData value) : data(value) {
    data.retain(data.context);
  }
  ~TextGeometry() { data.release(data.context); }
};
struct Document {
  SWTextDocument *pointer;
  explicit Document(SWTextDocument *value) : pointer(value) {
    sw_text_document_retain(pointer);
  }
  ~Document() { sw_text_document_release(pointer); }
};
struct Column {
  Key key;
  std::optional<std::string> name;
  int64_t header = 0;
};
struct Item {
  bool failed = false;
  std::optional<std::string> search;
  Key key;
  std::optional<Key> parentKey;
  int64_t index = -1, count = 0, span = 1, hierarchyLevel = -1;
  int64_t siblingIndex = -1, siblingCount = -1;
  int32_t enabled = 1, selected = -1, expanded = -1, focusable = 0;
  int32_t checked = -1, busy = 0;
  int32_t hasCells = 1;
  std::array<std::optional<std::string>, 4> text;
};
struct TextChild {
  int64_t node, start, end;
};
struct State {
  SWAccessibilityData data = {};
  // Name, description, automation id, value, error, URL.
  std::array<std::optional<std::string>, 6> text;
  std::array<std::vector<int64_t>, 8> relations;
  std::vector<TextChild> textChildren;
  std::shared_ptr<Hit> hit;
  std::shared_ptr<TextGeometry> textGeometry;
  std::shared_ptr<Document> document;
  uint64_t documentRevision = 0, documentEpoch = 0;
  int64_t count = -1, activeCollection = 0;
  int64_t hierarchyLevel = -1, siblingIndex = -1, siblingCount = -1;
  std::optional<Key> parentKey;
  uint64_t collectionRevision = 0;
  int32_t collectionActions = 0, selectionKind = -1;
  bool selectionRequired = false;
  bool hasCells = true;
  std::vector<Key> selected;
  std::vector<Column> columns;
  Key itemKey, cellKey, activeKey;
};
using States = std::map<int64_t, std::shared_ptr<const State>>;
struct Snapshot {
  States nodes;
  std::map<int64_t, std::vector<int64_t>> children;
  std::vector<int64_t> hitOrder;
  int64_t focused = 0;
};
struct Rect {
  double x = 0, y = 0, width = 0, height = 0;
};
struct Point {
  double x = 0, y = 0;
};
struct Change {
  std::shared_ptr<const Snapshot> before, after;
  std::vector<int64_t> changed, removed;
  bool structure = false;
};

class Tree;
class Element;
using ElementPtr = std::shared_ptr<Element>;

/** Platform calls stay native except for the explicit owner callback boundary.
 */
class Host {
public:
  virtual ~Host() = default;
  virtual bool onOwner() const = 0;
  virtual int32_t dispatch(const std::function<int32_t()> &body) = 0;
  virtual bool visible() const = 0;
  virtual bool enabled() const = 0;
  virtual bool focused() const = 0;
  virtual Rect bounds() const = 0;
  virtual std::optional<Rect> toScreen(Rect value) const = 0;
  virtual std::optional<Point> fromScreen(Point value) const = 0;
  virtual void changed(const Change &change) = 0;
  virtual void announce(int64_t node, const std::string &text,
                        int32_t priority) = 0;
  virtual void close() = 0;
};

class Tree : public std::enable_shared_from_this<Tree> {
public:
  explicit Tree(std::shared_ptr<Host> value, SWAccessibilityCallback action,
                void *context);
  std::shared_ptr<Host> host;
  const uint64_t identity;
  std::atomic<bool> closed{false};
  SWAccessibilityCallback callback;
  void *context;
  SWAccessibilityCollectionCallback collectionCallback = nullptr;
  void *collectionContext = nullptr;
  // Staging is accessed only by the existing window owner.
  std::map<int64_t, std::shared_ptr<State>> pending;
  std::set<int64_t> removed;
  bool publishing = false;

  std::shared_ptr<const Snapshot> snapshot() const;
  std::shared_ptr<const State> state(int64_t number) const;
  ElementPtr element(Reference ref);
  ElementPtr elementBySerial(uint64_t serial);
  ElementPtr actual(int64_t number);
  ElementPtr itemAt(int64_t owner, int64_t index);
  ElementPtr item(int64_t owner, const Key &key);
  ElementPtr cell(int64_t owner, const Key &row, const Key &column);
  ElementPtr column(int64_t owner, const Key &key);
  std::shared_ptr<const State> mounted(int64_t owner, const Key &row,
                                       const Key &column = {}) const;
  bool exposed(int64_t number, bool enabled = false) const;
  ElementPtr hit(Point point, int64_t ancestor = 0);
  int32_t query(int64_t owner, int32_t operation, int64_t index, const Key &key,
                Item *reply);
  int32_t perform(int64_t number, int32_t action,
                  const std::optional<std::string> &text = {},
                  int64_t start = -1, int64_t end = -1, uint64_t revision = 0,
                  double position = 0);
  int32_t operate(int64_t owner, const Key &key, int32_t operation);
  void commit(int64_t focused);
  void retire(int64_t number);
  void close();

private:
  mutable std::mutex mutex;
  std::shared_ptr<const Snapshot> published;
  std::set<int64_t> retiring;
  std::map<Reference, std::weak_ptr<Element>> elements;
  uint64_t nextElement = 0;
  void refreshProxies(const std::vector<ElementPtr> &proxies,
                      const std::shared_ptr<const Snapshot> &next);
};

class Element : public std::enable_shared_from_this<Element> {
public:
  Element(std::weak_ptr<Tree> owner, Reference ref, uint64_t value)
      : tree(std::move(owner)), reference(std::move(ref)), serial(value) {}
  std::weak_ptr<Tree> tree;
  const Reference reference;
  const uint64_t serial;
  std::atomic<bool> retired{false};
  std::shared_ptr<const State> state();
  bool exposed(bool enabled = false);
  ElementPtr parent();
  int64_t childCount();
  ElementPtr childAt(int64_t index);
  int64_t indexInParent();
  std::vector<ElementPtr> selection();
  ElementPtr selectionContainer();
  int32_t perform(int32_t action, const std::optional<std::string> &text = {},
                  int64_t start = -1, int64_t end = -1, uint64_t revision = 0,
                  double position = 0);
  int32_t realize();
  int64_t columnIndex() const;
  int64_t rowCount();
  int64_t columnCount();
  ElementPtr cellAt(int64_t row, int64_t column);
  std::vector<ElementPtr> headers(bool rows);

private:
  std::mutex mutex;
  uint64_t cachedRevision = 0;
  std::optional<Item> cached;
  std::vector<ElementPtr> actualChildren();
};

struct Handle {
  std::shared_ptr<Tree> tree;
};
bool readKey(Key &target, int32_t kind, int64_t number, const uint8_t *text,
             int64_t length);
int32_t actionCapability(int32_t action);
bool publicText(const State &state);
int64_t utf8Offset(const std::string &text, int64_t units, bool utf16);
int64_t textUnits(const std::string &text, int64_t bytes, bool utf16);
std::string textSlice(const std::string &text, int64_t start, int64_t end,
                      bool utf16);

} // namespace swax
#endif
