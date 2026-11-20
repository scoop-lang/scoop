#include "core.h"
#include <cerrno>
#include <stdexcept>

using namespace swax;
namespace swax {
bool readKey(Key &target, int32_t kind, int64_t number, const uint8_t *text,
             int64_t length) {
  if (kind == 0) {
    target = {0, number, {}};
    return true;
  }
  if (kind != 1 || length < 0 || (length && !text))
    return false;
  target = {1, 0,
            length ? std::string(reinterpret_cast<const char *>(text),
                                 static_cast<size_t>(length))
                   : ""};
  return true;
}
} // namespace swax
static Tree &tree(void *pointer) {
  return *static_cast<Handle *>(pointer)->tree;
}
static State &pending(void *pointer, int64_t number) {
  auto &owner = tree(pointer);
  if (owner.closed || !owner.publishing || !owner.host->onOwner())
    throw std::logic_error("no accessibility publication");
  auto found = owner.pending.find(number);
  if (found == owner.pending.end())
    throw std::logic_error("unknown accessibility node");
  return *found->second;
}
template <typename Body> static int64_t publish(Body body) noexcept {
  try {
    body();
    return 0;
  } catch (const std::bad_alloc &) {
    return ENOMEM;
  } catch (...) {
    return -4;
  }
}
static std::optional<std::string> textValue(const uint8_t *text,
                                            int64_t length) {
  if (length == -1 && !text)
    return {};
  if (length < 0 || (length && !text))
    throw std::invalid_argument("invalid accessibility text");
  return length ? std::string(reinterpret_cast<const char *>(text),
                              static_cast<size_t>(length))
                : "";
}
static Key keyValue(int32_t kind, int64_t number, const uint8_t *text,
                    int64_t length) {
  Key result;
  if (!readKey(result, kind, number, text, length))
    throw std::invalid_argument("invalid accessibility key");
  return result;
}

extern "C" {
SWAccessibilityBytes
scoop_window_accessibility_collection_search(void *pointer) {
  const auto &reply = *static_cast<Item *>(pointer);
  return reply.search
             ? SWAccessibilityBytes{reinterpret_cast<const uint8_t *>(
                                        reply.search->data()),
                                    static_cast<int64_t>(reply.search->size())}
             : SWAccessibilityBytes{nullptr, -1};
}
int64_t scoop_window_accessibility_begin(void *pointer) {
  return publish([&] {
    auto &owner = tree(pointer);
    if (owner.closed || owner.publishing || !owner.host->onOwner())
      throw std::logic_error("invalid accessibility begin");
    owner.pending.clear();
    owner.removed.clear();
    owner.publishing = true;
  });
}
int64_t scoop_window_accessibility_put(void *pointer,
                                       SWAccessibilityData data) {
  return publish([&] {
    auto &owner = tree(pointer);
    if (owner.closed || !owner.publishing || !owner.host->onOwner())
      throw std::logic_error("invalid accessibility put");
    auto value = std::make_shared<State>();
    value->data = data;
    owner.pending[data.number] = value;
  });
}
int64_t scoop_window_accessibility_text(void *pointer, int64_t number,
                                        int32_t field, const uint8_t *text,
                                        int64_t length) {
  return publish([&] {
    pending(pointer, number).text.at(field) = textValue(text, length);
  });
}
int64_t scoop_window_accessibility_relation(void *pointer, int64_t number,
                                            int32_t field, int64_t target) {
  return publish(
      [&] { pending(pointer, number).relations.at(field).push_back(target); });
}
int64_t scoop_window_accessibility_text_child(void *pointer, int64_t number,
                                              int64_t child, int64_t start,
                                              int64_t end) {
  return publish([&] {
    pending(pointer, number).textChildren.push_back({child, start, end});
  });
}
int64_t scoop_window_accessibility_hit(void *pointer, int64_t number,
                                       SWAccessibilityHitData value) {
  return publish(
      [&] { pending(pointer, number).hit = std::make_shared<Hit>(value); });
}
int64_t
scoop_window_accessibility_text_geometry(void *pointer, int64_t number,
                                         SWAccessibilityTextData value) {
  return publish([&] {
    pending(pointer, number).textGeometry =
        std::make_shared<TextGeometry>(value);
  });
}
int64_t scoop_window_accessibility_document(void *pointer, int64_t number,
                                            SWTextDocument *document,
                                            uint64_t revision, uint64_t epoch) {
  return publish([&] {
    auto &value = pending(pointer, number);
    if (document) {
      auto current = sw_text_document_state(document);
      if (!publicText(value) || !current.alive || !revision ||
          current.revision < revision ||
          (current.revision == revision &&
           current.length != static_cast<int64_t>(value.text[3]->size())))
        throw std::invalid_argument(
            "semantic document does not match published text");
      value.document = std::make_shared<Document>(document);
    }
    value.documentRevision = revision;
    value.documentEpoch = epoch;
  });
}
int64_t scoop_window_accessibility_remove(void *pointer, int64_t number) {
  return publish([&] {
    auto &owner = tree(pointer);
    if (!owner.publishing || !owner.host->onOwner())
      throw std::logic_error("invalid accessibility remove");
    owner.removed.insert(number);
  });
}
int64_t scoop_window_accessibility_commit(void *pointer, int64_t focused) {
  return publish([&] { tree(pointer).commit(focused); });
}
void scoop_window_accessibility_abort(void *pointer) {
  auto &owner = tree(pointer);
  owner.pending.clear();
  owner.removed.clear();
  owner.publishing = false;
}
void scoop_window_accessibility_retire(void *pointer, int64_t number) {
  tree(pointer).retire(number);
}
void scoop_window_accessibility_close(void *pointer) {
  auto *handle = static_cast<Handle *>(pointer);
  handle->tree->close();
  delete handle;
}
void scoop_window_accessibility_announce(void *pointer, int64_t number,
                                         const uint8_t *text, int64_t length,
                                         int32_t priority) {
  auto &owner = tree(pointer);
  auto value = owner.state(number);
  if (!value || !owner.exposed(number) ||
      (value->data.flags & (SWAXProtected | SWAXBusy)))
    return;
  try {
    owner.host->announce(number, textValue(text, length).value_or(""),
                         priority);
  } catch (
      ...) { /* A notification cannot roll back an already published state. */
  }
}

void scoop_window_accessibility_collection_callback(
    void *pointer, SWAccessibilityCollectionCallback callback, void *context) {
  tree(pointer).collectionCallback = callback;
  tree(pointer).collectionContext = context;
}
int64_t scoop_window_accessibility_collection(void *pointer, int64_t number,
                                              int64_t count, uint64_t revision,
                                              int32_t actions) {
  return publish([&] {
    auto &value = pending(pointer, number);
    value.count = count;
    value.collectionRevision = revision;
    value.collectionActions = actions;
  });
}
int64_t scoop_window_accessibility_collection_selection(void *pointer,
                                                        int64_t owner,
                                                        int32_t kind,
                                                        int32_t required) {
  return publish([&] {
    auto &value = pending(pointer, owner);
    value.selectionKind = kind;
    value.selectionRequired = required != 0;
  });
}
int64_t scoop_window_accessibility_collection_selected(
    void *pointer, int64_t owner, int32_t kind, int64_t number,
    const uint8_t *text, int64_t length) {
  return publish([&] {
    pending(pointer, owner)
        .selected.push_back(keyValue(kind, number, text, length));
  });
}
int64_t scoop_window_accessibility_collection_column(
    void *pointer, int64_t owner, int64_t index, int32_t kind, int64_t number,
    const uint8_t *text, int64_t length, int64_t header) {
  return publish([&] {
    auto &columns = pending(pointer, owner).columns;
    if (index != static_cast<int64_t>(columns.size()))
      throw std::invalid_argument("invalid semantic column index");
    columns.push_back({keyValue(kind, number, text, length), {}, header});
  });
}
int64_t scoop_window_accessibility_collection_column_name(void *pointer,
                                                          int64_t owner,
                                                          int64_t index,
                                                          const uint8_t *text,
                                                          int64_t length) {
  return publish([&] {
    pending(pointer, owner).columns.at(index).name = textValue(text, length);
  });
}
int64_t scoop_window_accessibility_collection_row(void *pointer, int64_t node,
                                                  int32_t kind, int64_t number,
                                                  const uint8_t *text,
                                                  int64_t length) {
  return publish([&] {
    pending(pointer, node).itemKey = keyValue(kind, number, text, length);
  });
}
int64_t scoop_window_accessibility_collection_cell(void *pointer, int64_t node,
                                                   int32_t kind, int64_t number,
                                                   const uint8_t *text,
                                                   int64_t length) {
  return publish([&] {
    pending(pointer, node).cellKey = keyValue(kind, number, text, length);
  });
}
int64_t scoop_window_accessibility_collection_active(
    void *pointer, int64_t owner, int64_t collection, int32_t kind,
    int64_t number, const uint8_t *text, int64_t length) {
  return publish([&] {
    auto &value = pending(pointer, owner);
    value.activeCollection = collection;
    value.activeKey = keyValue(kind, number, text, length);
  });
}
void scoop_window_accessibility_collection_item(
    void *pointer, int64_t index, int64_t count, int64_t span, int32_t enabled,
    int32_t selected, int32_t expanded, int32_t focusable) {
  auto &reply = *static_cast<Item *>(pointer);
  reply.index = index;
  reply.count = count;
  reply.span = span;
  reply.enabled = enabled;
  reply.selected = selected;
  reply.expanded = expanded;
  reply.focusable = focusable;
}
void scoop_window_accessibility_collection_item_key(void *pointer, int32_t kind,
                                                    int64_t number,
                                                    const uint8_t *text,
                                                    int64_t length) {
  auto &reply = *static_cast<Item *>(pointer);
  try {
    reply.key = keyValue(kind, number, text, length);
  } catch (...) {
    reply.failed = true;
  }
}
void scoop_window_accessibility_collection_item_text(void *pointer,
                                                     int32_t field,
                                                     const uint8_t *text,
                                                     int64_t length) {
  auto &reply = *static_cast<Item *>(pointer);
  try {
    reply.text.at(field) = textValue(text, length);
  } catch (...) {
    reply.failed = true;
  }
}
} // extern "C"
