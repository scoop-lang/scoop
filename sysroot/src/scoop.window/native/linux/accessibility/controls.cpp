#include "provider.h"
#include <algorithm>

namespace swax::atspi {
std::vector<Action> actions(const State &state) {
  static const Action possible[] = {
      {"click", 0},           {"toggle", 1},          {"show-menu", 7},
      {"scroll-backward", 9}, {"scroll-forward", 10}, {"scroll-to-visible", 11},
      {"increment", 13},      {"decrement", 14},      {"select", 15},
      {"deselect", 16},       {"expand", 17},         {"collapse", 18},
      {"dismiss", 19}};
  std::vector<Action> result;
  for (const auto &action : possible) {
    if (!(state.data.actions & actionCapability(action.code)))
      continue;
    if ((action.code == 15 && state.data.selected == 1) ||
        (action.code == 16 && state.data.selected == 0) ||
        (action.code == 17 && state.data.expanded == 1) ||
        (action.code == 18 && state.data.expanded == 0))
      continue;
    result.push_back(action);
  }
  return result;
}
bool selected(const State &state) {
  return state.data.selected == 1 ||
         (state.data.role == 9 && state.data.checked == 1);
}
bool select(const ElementPtr &element, bool value) {
  auto state = element ? element->state() : nullptr;
  if (!state)
    unavailable();
  const bool current = selected(*state);
  if (current == value)
    return true;
  auto parent = element->selectionContainer();
  auto container = parent ? parent->state() : nullptr;
  if (!value && current && container && container->selectionRequired &&
      parent->selection().size() <= 1)
    return false;
  if (state->data.role == 9)
    return value ? actionResult(element->perform(0)) : false;
  return actionResult(element->perform(value ? 15 : 16));
}
GVariant *control(Request &request, const char *interface, const char *method,
                  GVariant *parameters) {
  if (!strcmp(interface, "org.a11y.atspi.Action")) {
    auto values = actions(*request.state);
    if (!strcmp(method, "GetActions")) {
      GVariantBuilder result;
      g_variant_builder_init(&result, G_VARIANT_TYPE("a(sss)"));
      for (const auto &value : values)
        g_variant_builder_add(&result, "(sss)", value.name, "", "");
      return g_variant_new("(@a(sss))", g_variant_builder_end(&result));
    }
    int index = -1;
    g_variant_get(parameters, "(i)", &index);
    if (index < 0 || static_cast<size_t>(index) >= values.size())
      invalid("Action index is out of range");
    if (!strcmp(method, "GetName") || !strcmp(method, "GetLocalizedName"))
      return g_variant_new("(s)", values[index].name);
    if (!strcmp(method, "GetDescription") || !strcmp(method, "GetKeyBinding"))
      return g_variant_new("(s)", "");
    if (!strcmp(method, "DoAction"))
      return g_variant_new(
          "(b)", actionResult(request.element->perform(values[index].code)));
    unsupported();
  }
  if (!strcmp(method, "SelectAll")) {
    if (request.state->selectionKind != 1)
      return g_variant_new("(b)", FALSE);
    const auto count = request.element->childCount();
    for (int64_t index = 0; index < count; ++index)
      if (!select(request.element->childAt(index), true))
        return g_variant_new("(b)", FALSE);
    return g_variant_new("(b)", TRUE);
  }
  if (!strcmp(method, "ClearSelection")) {
    if (request.state->selectionRequired)
      return g_variant_new("(b)", FALSE);
    for (const auto &child : request.element->selection())
      if (!select(child, false))
        return g_variant_new("(b)", FALSE);
    return g_variant_new("(b)", TRUE);
  }
  int index = -1;
  g_variant_get(parameters, "(i)", &index);
  if (!strcmp(method, "GetSelectedChild") ||
      !strcmp(method, "DeselectSelectedChild")) {
    auto selection = request.element->selection();
    if (index < 0 || static_cast<size_t>(index) >= selection.size())
      invalid("Selected child index is out of range");
    if (!strcmp(method, "GetSelectedChild"))
      return g_variant_new(
          "(@(so))", request.host->reference(selection[index], request.sender));
    return g_variant_new("(b)", select(selection[index], false));
  }
  if (index < 0 || index >= request.element->childCount())
    invalid("Child index is out of range");
  auto child = request.element->childAt(index);
  if (!child)
    unavailable();
  if (!strcmp(method, "SelectChild"))
    return g_variant_new("(b)", select(child, true));
  if (!strcmp(method, "DeselectChild"))
    return g_variant_new("(b)", select(child, false));
  if (!strcmp(method, "IsChildSelected")) {
    auto state = child->state();
    if (!state)
      unavailable();
    return g_variant_new("(b)", selected(*state));
  }
  unsupported();
}
} // namespace swax::atspi
