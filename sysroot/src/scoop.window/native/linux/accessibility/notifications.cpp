#include "provider.h"
#include <algorithm>
#include <cmath>

namespace swax::atspi {
void WindowHost::signal(const ElementPtr &element, const char *name,
                        const char *detail, int a, int b, GVariant *value) {
  if (!bus || !element || !window)
    return;
  GError *error = nullptr;
  const auto objectPath = exportPath(element);
  if (!g_dbus_connection_emit_signal(
          bus, nullptr, objectPath.c_str(), "org.a11y.atspi.Event.Object", name,
          g_variant_new("(siiva{sv})", detail, a, b,
                        value ? value : g_variant_new_int32(0), nullptr),
          &error)) {
    g_clear_error(&error);
    fail(EIO);
  }
}
static void cacheSignal(WindowHost &host, const char *name, GVariant *value) {
  GError *error = nullptr;
  GVariant *parameters = g_variant_new_tuple(&value, 1);
  if (!g_dbus_connection_emit_signal(host.bus, nullptr, "/org/a11y/atspi/cache",
                                     "org.a11y.atspi.Cache", name, parameters,
                                     &error)) {
    g_clear_error(&error);
    host.fail(EIO);
  }
}
static GVariant *eventText(const std::optional<std::string> &value) {
  if (value && value->find('\0') != std::string::npos)
    return nullptr;
  return string(value.value_or(""));
}
void WindowHost::scheduleCollection(int64_t number) {
  pendingCollections.insert(number);
  if (notificationSource)
    return;
  notificationSource = g_idle_source_new();
  g_source_set_callback(
      notificationSource,
      [](void *data) -> gboolean {
        auto host = static_cast<std::weak_ptr<WindowHost> *>(data)->lock();
        if (!host)
          return G_SOURCE_REMOVE;
        auto *source = host->notificationSource;
        host->notificationSource = nullptr;
        if (source)
          g_source_unref(source);
        try {
          host->refreshCollections();
        } catch (const std::bad_alloc &) {
          host->fail(ENOMEM);
        } catch (...) {
          host->fail(EIO);
        }
        return G_SOURCE_REMOVE;
      },
      new std::weak_ptr<WindowHost>(shared_from_this()),
      [](void *data) {
        delete static_cast<std::weak_ptr<WindowHost> *>(data);
      });
  g_source_attach(notificationSource, context);
}
void WindowHost::refreshCollections() {
  auto changed = std::move(pendingCollections);
  pendingCollections.clear();
  auto owner = tree.lock();
  if (!bus || !window || !owner || owner->closed)
    return;
  std::vector<ElementPtr> values;
  for (const auto &entry : exports)
    if (entry.second.element->reference.kind &&
        changed.count(entry.second.element->reference.node))
      values.push_back(entry.second.element);
  for (const auto &element : values) {
    if (element->retired)
      continue;
    try {
      auto state = element->state();
      if (!state)
        continue;
      // Only already-issued identities are refreshed, after the Scoop
      // publication has returned.
      try {
        cacheSignal(*this, "AddAccessible", cacheItem(element, nullptr));
      } catch (const Failure &) {
        // The later getters report protocol representability limits.
      }
      signal(element, "PropertyChange", "accessible-name", 0, 0,
             eventText(state->text[0]));
      signal(element, "PropertyChange", "accessible-description", 0, 0,
             eventText(state->text[1]));
      signal(element, "AttributesChanged", "", 0, 0, string(""));
      signal(element, "VisibleDataChanged", "", 0, 0, nullptr);
    } catch (const Failure &) {
      // Unrepresentable data remains explicitly unavailable when the client
      // queries it.
    }
  }
}
static void textChanged(WindowHost &host, const ElementPtr &element,
                        const State &before, const State &after) {
  if (!publicText(after) || before.text[3] == after.text[3])
    return;
  const auto oldText = publicText(before) ? before.text[3].value_or("") : "";
  const auto &newText = *after.text[3];
  if (oldText.find('\0') != std::string::npos ||
      newText.find('\0') != std::string::npos) {
    host.signal(element, "VisibleDataChanged", "", 0, 0, nullptr);
    return;
  }
  size_t prefix = 0;
  while (prefix < oldText.size() && prefix < newText.size() &&
         oldText[prefix] == newText[prefix])
    ++prefix;
  while (prefix && prefix < newText.size() &&
         (static_cast<uint8_t>(newText[prefix]) & 0xc0) == 0x80)
    --prefix;
  size_t oldEnd = oldText.size(), newEnd = newText.size();
  while (oldEnd > prefix && newEnd > prefix &&
         oldText[oldEnd - 1] == newText[newEnd - 1]) {
    --oldEnd;
    --newEnd;
  }
  while (newEnd < newText.size() &&
         (static_cast<uint8_t>(newText[newEnd]) & 0xc0) == 0x80) {
    ++oldEnd;
    ++newEnd;
  }
  auto start = integer(textUnits(newText, prefix, false));
  if (oldEnd > prefix) {
    auto removed = oldText.substr(prefix, oldEnd - prefix);
    host.signal(element, "TextChanged", "delete", start,
                integer(textUnits(removed, removed.size(), false)),
                string(removed));
  }
  if (newEnd > prefix) {
    auto inserted = newText.substr(prefix, newEnd - prefix);
    host.signal(element, "TextChanged", "insert", start,
                integer(textUnits(inserted, inserted.size(), false)),
                string(inserted));
  }
}
static std::vector<int64_t> children(const Snapshot &snapshot, int64_t parent) {
  std::vector<int64_t> result;
  std::function<void(int64_t)> append = [&](int64_t id) {
    auto list = snapshot.children.find(id);
    if (list == snapshot.children.end())
      return;
    for (auto child : list->second) {
      const auto found = snapshot.nodes.find(child);
      if (found == snapshot.nodes.end() ||
          (found->second->data.flags & SWAXHidden))
        continue;
      const auto &state = *found->second;
      if ((state.data.flags & SWAXIgnored) && !state.itemKey)
        append(child);
      else
        result.push_back(child);
    }
  };
  append(parent);
  return result;
}
static ElementPtr oldElement(WindowHost &host, int64_t number) {
  for (const auto &entry : host.exports)
    if (!entry.second.element->reference.kind &&
        entry.second.element->reference.node == number)
      return entry.second.element;
  return {};
}
static void structureChanged(WindowHost &host, const Change &change,
                             const std::shared_ptr<Tree> &owner) {
  std::set<int64_t> parents;
  for (const auto *snapshot : {change.before.get(), change.after.get()})
    for (const auto &entry : snapshot->children)
      parents.insert(entry.first);
  for (auto parent : parents) {
    try {
      auto state = owner->state(parent);
      if (!state || state->count >= 0 ||
          (state->data.flags & (SWAXHidden | SWAXIgnored)))
        continue;
      auto before = children(*change.before, parent),
           after = children(*change.after, parent);
      if (before == after)
        continue;
      auto target = owner->actual(parent);
      for (size_t index = before.size(); index > 0; --index)
        if (index > after.size() || before[index - 1] != after[index - 1])
          host.signal(target, "ChildrenChanged", "remove", integer(index - 1),
                      0, host.reference(oldElement(host, before[index - 1])));
      for (size_t index = 0; index < after.size(); ++index)
        if (index >= before.size() || before[index] != after[index])
          host.signal(target, "ChildrenChanged", "add", integer(index), 0,
                      host.reference(owner->actual(after[index])));
    } catch (const Failure &) {
      // One container outside the protocol's integer range must not suppress
      // notifications for other containers or retiring objects.
    }
  }
}
void WindowHost::changed(const Change &change) {
  if (!bus || !window)
    return;
  auto owner = tree.lock();
  if (!owner || owner->closed)
    return;
  try {
    if (change.structure)
      structureChanged(*this, change, owner);
    for (auto number : change.changed) {
      try {
        const auto next = change.after->nodes.find(number),
                   previous = change.before->nodes.find(number);
        if (next == change.after->nodes.end())
          continue;
        auto element = owner->actual(number);
        if (!element)
          continue;
        const auto &after = *next->second;
        if (!element->reference.kind) {
          try {
            cacheSignal(*this, "AddAccessible", cacheItem(element, nullptr));
          } catch (const Failure &) {
            // D-Bus cannot cache every valid Scoop string or an out-of-range
            // collection size.
          }
        }
        if (previous == change.before->nodes.end())
          continue;
        const auto &before = *previous->second;
        const auto &a = before.data, &b = after.data;
        if (before.text[0] != after.text[0])
          signal(element, "PropertyChange", "accessible-name", 0, 0,
                 eventText(after.text[0]));
        if (before.text[1] != after.text[1] || before.text[4] != after.text[4])
          signal(element, "PropertyChange", "accessible-description", 0, 0,
                 eventText(after.text[1]));
        struct Flag {
          int flag;
          const char *name;
        };
        const Flag flags[] = {
            {SWAXEnabled, "enabled"},       {SWAXBusy, "busy"},
            {SWAXInvalid, "invalid-entry"}, {SWAXRequired, "required"},
            {SWAXModal, "modal"},           {SWAXReadOnly, "read-only"}};
        for (const auto &flag : flags)
          if ((a.flags ^ b.flags) & flag.flag)
            signal(element, "StateChanged", flag.name,
                   (b.flags & flag.flag) != 0, 0, nullptr);
        if (a.checked != b.checked) {
          signal(element, "StateChanged", "checked", b.checked == 1, 0,
                 nullptr);
          if ((a.checked == 2) != (b.checked == 2))
            signal(element, "StateChanged", "indeterminate", b.checked == 2, 0,
                   nullptr);
        }
        if (a.expanded != b.expanded)
          signal(element, "StateChanged", "expanded", b.expanded == 1, 0,
                 nullptr);
        if (a.selected != b.selected)
          signal(element, "StateChanged", "selected", b.selected == 1, 0,
                 nullptr);
        if (before.selected != after.selected || a.selected != b.selected)
          signal(element, "SelectionChanged", "", 0, 0, nullptr);
        if (a.number_value != b.number_value ||
            a.scroll_offset != b.scroll_offset)
          signal(element, "PropertyChange", "accessible-value", 0, 0,
                 g_variant_new_double(b.role == 6 ? b.scroll_offset
                                                  : b.number_value));
        textChanged(*this, element, before, after);
        if (publicText(after) && a.cursor != b.cursor && b.cursor >= 0)
          signal(element, "TextCaretMoved", "",
                 integer(textUnits(*after.text[3], b.cursor, false)), 0,
                 nullptr);
        if (publicText(after) && (a.anchor != b.anchor || a.cursor != b.cursor))
          signal(element, "TextSelectionChanged", "", 0, 0, nullptr);
        if (before.collectionRevision != after.collectionRevision) {
          scheduleCollection(number);
          signal(element, "ModelChanged", "", 0, 0, nullptr);
          signal(element, "VisibleDataChanged", "", 0, 0, nullptr);
        }
        if (before.activeCollection != after.activeCollection ||
            before.activeKey != after.activeKey ||
            before.relations[7] != after.relations[7]) {
          auto active =
              after.activeCollection && after.activeKey
                  ? owner->item(after.activeCollection, after.activeKey)
              : after.relations[7].empty()
                  ? nullptr
                  : owner->actual(after.relations[7][0]);
          signal(element, "ActiveDescendantChanged", "", 0, 0,
                 reference(active));
        }
        if (a.geometry != b.geometry || a.visible != b.visible || a.x != b.x ||
            a.y != b.y || a.width != b.width || a.height != b.height) {
          signal(element, "VisibleDataChanged", "", 0, 0, nullptr);
          if (a.visible != b.visible)
            signal(element, "StateChanged", "showing", b.visible && visible(),
                   0, nullptr);
        }
      } catch (const Failure &) {
        // Getters report unrepresentable values for this node. Continue the
        // same publication for every other changed identity.
      }
    }
    if (change.before->focused != change.after->focused) {
      if (change.before->focused) {
        auto previous = owner->actual(change.before->focused);
        signal(previous ? previous : oldElement(*this, change.before->focused),
               "StateChanged", "focused", 0, 0, nullptr);
      }
      if (change.after->focused && focused())
        signal(owner->actual(change.after->focused), "StateChanged", "focused",
               1, 0, nullptr);
    }
    for (auto it = exports.begin(); it != exports.end();) {
      const auto element = it->second.element;
      if (element->retired) {
        signal(element, "StateChanged", "defunct", 1, 0, nullptr);
        cacheSignal(*this, "RemoveAccessible", reference(element));
        it = exports.erase(it);
      } else
        ++it;
    }
  } catch (const Failure &) {
    // A protocol representability limit is reported by getters, not an
    // application failure.
  } catch (const std::bad_alloc &) {
    fail(ENOMEM);
  } catch (...) {
    fail(EIO);
  }
}
void WindowHost::announce(int64_t node, const std::string &text,
                          int32_t priority) {
  if (text.find('\0') != std::string::npos)
    return;
  try {
    auto owner = tree.lock();
    auto element = owner && !owner->closed ? owner->actual(node) : nullptr;
    if (element && element->exposed())
      signal(element, "Announcement", "", priority, 0, string(text));
  } catch (const std::bad_alloc &) {
    fail(ENOMEM);
  } catch (...) {
    fail(EIO);
  }
}
} // namespace swax::atspi
extern "C" void sw_accessibility_geometry(SWWindow *window) {
  auto *host = static_cast<swax::atspi::WindowHost *>(window->accessibility);
  if (!host || !host->bus || !host->root)
    return;
  // Geometry uses current native coordinates; listeners are notified by the
  // existing window path.
  auto bounds = host->bounds();
  const auto &snapshot = window->snapshot;
  auto same = [](double a, double b) {
    return a == b || (std::isnan(a) && std::isnan(b));
  };
  auto *surface = sw_surface(window);
  const auto scale = surface ? gdk_surface_get_scale(surface) : 1.0;
  if (host->nativeGeometryKnown && same(host->nativeX, snapshot.x) &&
      same(host->nativeY, snapshot.y) && host->nativeWidth == bounds.width &&
      host->nativeHeight == bounds.height &&
      host->nativeVisible == snapshot.visible && host->nativeScale == scale)
    return;
  host->nativeGeometryKnown = true;
  host->nativeScale = scale;
  host->nativeX = snapshot.x;
  host->nativeY = snapshot.y;
  host->nativeWidth = bounds.width;
  host->nativeHeight = bounds.height;
  host->nativeVisible = snapshot.visible;
  try {
    host->signal(host->root, "VisibleDataChanged", "", 0, 0, nullptr);
  } catch (const std::bad_alloc &) {
    host->fail(ENOMEM);
  } catch (...) {
    host->fail(EIO);
  }
}
