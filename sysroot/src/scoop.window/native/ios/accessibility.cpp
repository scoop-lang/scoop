#include "accessibility.h"
#include "../accessibility/core.h"
#include <cerrno>

namespace {
using namespace swax;

SWAccessibilityBytes bytes(const std::string &text) {
  return {reinterpret_cast<const uint8_t *>(text.data()),
          static_cast<int64_t>(text.size())};
}

bool navigable(const State &state) {
  const auto &data = state.data;
  if (!data.visible || (data.flags & (SWAXHidden | SWAXIgnored)))
    return false;
  return data.role != 27 && (data.role != 0 || data.actions || state.text[0]);
}

class UIKitHost final : public Host {
public:
  void *projection = nullptr;
  ~UIKitHost() override { close(); }
  bool onOwner() const override { return swi_ax_on_main(); }
  int32_t dispatch(const std::function<int32_t()> &body) override {
    return onOwner() ? body() : 6;
  }
  bool visible() const override { return swi_ax_query(projection, SWIVisible); }
  bool enabled() const override { return swi_ax_query(projection, SWIEnabled); }
  bool focused() const override { return swi_ax_query(projection, SWIFocused); }
  Rect bounds() const override {
    SWIAccessibilityRect value{};
    swi_ax_geometry(projection, SWIViewBounds, &value);
    return {value.x, value.y, value.width, value.height};
  }
  std::optional<Rect> toScreen(Rect rect) const override {
    SWIAccessibilityRect value{rect.x, rect.y, rect.width, rect.height};
    if (!swi_ax_geometry(projection, SWIToScreen, &value))
      return {};
    return Rect{value.x, value.y, value.width, value.height};
  }
  std::optional<Point> fromScreen(Point point) const override {
    SWIAccessibilityRect value{point.x, point.y, 0, 0};
    if (!swi_ax_geometry(projection, SWIFromScreen, &value))
      return {};
    return Point{value.x, value.y};
  }
  void changed(const Change &change) override {
    bool reorder = change.structure;
    for (auto number : change.removed)
      swi_ax_remove(projection, number);
    for (auto number : change.changed) {
      const auto &value = *change.after->nodes.at(number);
      auto previous = change.before->nodes.find(number);
      reorder |= previous == change.before->nodes.end() ||
                 navigable(*previous->second) != navigable(value);
      SWAccessibilityBytes fields[6]{};
      for (size_t index = 0; index < value.text.size(); ++index)
        if (value.text[index])
          fields[index] = bytes(*value.text[index]);
      swi_ax_update(projection, &value.data, fields);
    }
    if (!reorder) {
      if (change.before->focused != change.after->focused)
        swi_ax_focus(projection, change.after->focused);
      return;
    }
    std::vector<int64_t> order, stack{0};
    while (!stack.empty()) {
      auto number = stack.back();
      stack.pop_back();
      const auto &state = *change.after->nodes.at(number);
      if (state.data.flags & SWAXHidden)
        continue;
      if (number && navigable(state))
        order.push_back(number);
      auto children = change.after->children.find(number);
      if (children != change.after->children.end())
        stack.insert(stack.end(), children->second.rbegin(),
                     children->second.rend());
    }
    swi_ax_order(projection, order.data(), order.size(), change.after->focused);
  }
  void announce(int64_t node, const std::string &text,
                int32_t priority) override {
    swi_ax_announce(projection, bytes(text), priority);
  }
  void close() override {
    if (projection) {
      swi_ax_detach(projection);
      projection = nullptr;
    }
  }
};
struct Result {
  void *pointer;
  int64_t error;
};
} // namespace

extern "C" Result
scoop_window_accessibility_create(void *scene, SWAccessibilityCallback callback,
                                  void *context) {
  if (!scene || !callback || !swi_ax_on_main())
    return {nullptr, -4};
  try {
    auto host = std::make_shared<UIKitHost>();
    auto handle = std::make_unique<swax::Handle>();
    handle->tree = std::make_shared<swax::Tree>(host, callback, context);
    host->projection = swi_ax_attach(scene, handle->tree.get());
    if (!host->projection)
      return {nullptr, -4};
    return {handle.release(), 0};
  } catch (const std::bad_alloc &) {
    return {nullptr, ENOMEM};
  }
}

int swi_ax_perform(void *pointer, int64_t number, int32_t action) {
  if (!pointer || !swi_ax_on_main())
    return 0;
  auto tree = static_cast<swax::Tree *>(pointer)->shared_from_this();
  return tree->perform(number, action) == 1;
}

int swi_ax_scroll(void *pointer, int64_t number, int axis, int forward) {
  if (!pointer || !swi_ax_on_main())
    return 0;
  auto tree = static_cast<swax::Tree *>(pointer)->shared_from_this();
  const int action = forward ? 10 : 9;
  auto scroll = [&](const swax::State &bar) {
    const auto &data = bar.data;
    return data.role == 6 && (data.flags & SWAXHasScrollRange) &&
           !(data.flags & (SWAXHidden | SWAXIgnored)) &&
           (axis < 0 || data.scroll_orientation == axis) &&
           (data.actions & swax::actionCapability(action)) &&
           tree->perform(data.number, action) == 1;
  };
  const auto snapshot = tree->snapshot();
  while (auto state = tree->state(number)) {
    if (state->data.role == 6 && scroll(*state))
      return 1;
    if (state->data.role == 5 || state->data.role == 4) {
      auto bars = snapshot->children.find(number);
      if (bars != snapshot->children.end())
        for (auto child : bars->second)
          if (auto bar = tree->state(child); bar && scroll(*bar))
            return 1;
    }
    if (!number)
      break;
    number = state->data.parent;
  }
  return 0;
}
