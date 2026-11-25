#include "provider.h"
#include <algorithm>

namespace swax::atspi {
uint32_t role(const State &state) {
  static const uint32_t roles[] = {99, 61, 43, 7,  79, 49, 48, 130, 62, 44, 99,
                                   27, 31, 88, 83, 51, 42, 33, 35,  38, 37, 16,
                                   55, 56, 47, 10, 62, 50, 90, 65,  91};
  if (state.data.role == 29 &&
      (!state.columns.empty() || state.data.table_columns > 0))
    return 66;
  if (state.data.role == 4 && (state.data.flags & SWAXProtected))
    return 40;
  if (state.data.role == 4 && state.data.expanded >= 0)
    return 11;
  if (state.data.role == 2 && state.data.expanded >= 0)
    return 129;
  if (state.data.role == 18 && state.data.checked >= 0)
    return 8;
  return state.data.role >= 0 &&
                 state.data.role < static_cast<int>(G_N_ELEMENTS(roles))
             ? roles[state.data.role]
             : 67;
}
const char *roleName(const State &state) {
  static const char *roles[] = {
      "grouping",      "text",          "push button",   "check box",
      "entry",         "scroll pane",   "scroll bar",    "switch",
      "toggle button", "radio button",  "grouping",      "image",
      "list",          "link",          "heading",       "slider",
      "progress bar",  "menu",          "menu item",     "page tab list",
      "page tab",      "dialog",        "table",         "table cell",
      "row header",    "column header", "toggle button", "separator",
      "table row",     "tree",          "tree item"};
  if (state.data.role == 29 &&
      (!state.columns.empty() || state.data.table_columns > 0))
    return "tree table";
  if (state.data.role == 4 && (state.data.flags & SWAXProtected))
    return "password text";
  if (state.data.role == 4 && state.data.expanded >= 0)
    return "combo box";
  if (state.data.role == 2 && state.data.expanded >= 0)
    return "push button menu";
  if (state.data.role == 18 && state.data.checked >= 0)
    return "check menu item";
  return state.data.role >= 0 &&
                 state.data.role < static_cast<int>(G_N_ELEMENTS(roles))
             ? roles[state.data.role]
             : "unknown";
}
std::optional<TextChild> textChild(const ElementPtr &element,
                                   std::shared_ptr<const State> &parent) {
  if (element->reference.kind)
    return {};
  auto owner = element->tree.lock();
  if (!owner)
    return {};
  auto current = element->state();
  while (current && current->data.number) {
    parent = owner->state(current->data.parent);
    if (!parent)
      return {};
    if (publicText(*parent)) {
      for (const auto &child : parent->textChildren)
        if (child.node == element->reference.node)
          return child;
    }
    current = parent;
  }
  return {};
}
std::vector<std::string> interfaces(const ElementPtr &element,
                                    const State &state) {
  std::vector<std::string> result = {"org.a11y.atspi.Accessible",
                                     "org.a11y.atspi.Component"};
  if (!element->reference.kind && !element->reference.node)
    result.push_back("org.a11y.atspi.Socket");
  if (!actions(state).empty())
    result.push_back("org.a11y.atspi.Action");
  if (state.data.has_range || state.data.role == 6)
    result.push_back("org.a11y.atspi.Value");
  if (state.selectionKind >= 0)
    result.push_back("org.a11y.atspi.Selection");
  if (publicText(state)) {
    result.push_back("org.a11y.atspi.Text");
    if (!state.textChildren.empty())
      result.push_back("org.a11y.atspi.Hypertext");
  }
  if (state.data.role == 4 && !(state.data.flags & SWAXReadOnly) &&
      (state.data.actions & (4 | 32)))
    result.push_back("org.a11y.atspi.EditableText");
  if (state.data.role == 22 ||
      (state.data.role == 29 &&
       (!state.columns.empty() || state.data.table_columns > 0)))
    result.push_back("org.a11y.atspi.Table");
  if (state.data.cell_row >= 0 && state.data.cell_column >= 0)
    result.push_back("org.a11y.atspi.TableCell");
  std::shared_ptr<const State> parent;
  if (state.data.role == 13 || textChild(element, parent))
    result.push_back("org.a11y.atspi.Hyperlink");
  return result;
}
GVariant *states(const ElementPtr &element, const State &state) {
  uint64_t result = 0;
  auto set = [&](int bit, bool value) {
    if (value)
      result |= uint64_t{1} << bit;
  };
  auto owner = element->tree.lock();
  const auto &data = state.data;
  bool exposed = element->exposed(), enabled = element->exposed(true);
  set(3, data.flags & SWAXBusy);
  set(4, data.checked == 1);
  set(5, data.expanded == 0);
  set(6, element->retired || !owner || owner->closed);
  set(7, data.role == 4 && !(data.flags & SWAXReadOnly) &&
             (data.actions & (4 | 32)));
  set(8, enabled);
  set(24, enabled);
  set(9, data.expanded >= 0);
  set(10, data.expanded == 1);
  set(11, data.actions & 8);
  auto snapshot = owner ? owner->snapshot() : nullptr;
  set(12, snapshot && snapshot->focused && snapshot->focused == data.number &&
              owner->host->focused());
  set(14, data.scroll_orientation == 1);
  set(29, data.scroll_orientation == 2);
  set(16, data.flags & SWAXModal);
  set(17, data.role == 4 && (data.flags & SWAXMultiline));  // MULTI_LINE
  set(26, data.role == 4 && !(data.flags & SWAXMultiline)); // SINGLE_LINE
  set(18, state.selectionKind == 1);
  set(20, (data.role == 8 || data.role == 26) && data.checked == 1);
  set(22, data.selected >= 0 || data.role == 9);
  set(23, selected(state));
  set(25, exposed && (data.visible ||
                      (!element->reference.kind && !element->reference.node)));
  set(30, exposed);
  set(31, state.count >= 0);
  set(32, data.checked == 2 || (data.role == 16 && !data.has_range));
  set(33, data.flags & SWAXRequired);
  set(36, data.flags & SWAXInvalid);
  set(37, data.role == 4 && data.expanded >= 0);
  set(38, publicText(state) && (data.actions & 16));
  set(41, data.checked >= 0);
  set(42, data.expanded >= 0 &&
              (data.role == 2 || data.role == 4 || data.role == 18));
  set(43, data.flags & SWAXReadOnly);
  GVariantBuilder values;
  g_variant_builder_init(&values, G_VARIANT_TYPE("au"));
  g_variant_builder_add(&values, "u", static_cast<uint32_t>(result));
  g_variant_builder_add(&values, "u", static_cast<uint32_t>(result >> 32));
  return g_variant_builder_end(&values);
}
std::map<std::string, std::string> objectAttributes(const State &state) {
  std::map<std::string, std::string> result = {{"toolkit", "Scoop"}};
  if (state.data.heading_level > 0)
    result["level"] = std::to_string(state.data.heading_level);
  if (state.hierarchyLevel >= 0)
    result["level"] =
        std::to_string(static_cast<uint64_t>(state.hierarchyLevel) + 1);
  if (state.siblingIndex >= 0)
    result["posinset"] = std::to_string(state.siblingIndex + 1);
  if (state.siblingCount >= 0)
    result["setsize"] = std::to_string(state.siblingCount);
  if (state.text[2])
    result["id"] = *state.text[2];
  if (state.data.live)
    result["live"] = state.data.live == 2 ? "assertive" : "polite";
  if (state.data.flags & SWAXBusy)
    result["busy"] = "true";
  if (state.data.flags & SWAXInvalid)
    result["invalid"] = "true";
  if (state.count >= 0)
    result["setsize"] = std::to_string(state.count);
  return result;
}
static GVariant *relations(Request &request) {
  GVariantBuilder result;
  g_variant_builder_init(&result, G_VARIANT_TYPE("a(ua(so))"));
  const uint32_t relation[] = {2, 3, 1, 18, 21};
  auto owner = request.element->tree.lock();
  for (size_t index = 0; index < G_N_ELEMENTS(relation); ++index) {
    std::vector<ElementPtr> nodes;
    for (auto id : request.state->relations[index]) {
      auto target = owner->actual(id);
      if (target && target->exposed())
        nodes.push_back(target);
    }
    if (!nodes.empty())
      g_variant_builder_add(&result, "(u@a(so))", relation[index],
                            request.host->references(nodes, request.sender));
  }
  return g_variant_builder_end(&result);
}
GVariant *accessible(Request &request, const char *method,
                     GVariant *parameters) {
  auto &element = request.element;
  auto &state = *request.state;
  if (!strcmp(method, "GetChildAtIndex")) {
    int index = -1;
    g_variant_get(parameters, "(i)", &index);
    if (index < 0 || index >= element->childCount())
      invalid("Child index is out of range");
    auto child = element->childAt(index);
    if (!child)
      unavailable();
    return g_variant_new("(@(so))",
                         request.host->reference(child, request.sender));
  }
  if (!strcmp(method, "GetChildren")) {
    GVariantBuilder children;
    g_variant_builder_init(&children, G_VARIANT_TYPE("a(so)"));
    const auto count = element->childCount();
    for (int64_t index = 0; index < count; ++index) {
      auto child = element->childAt(index);
      if (!child) {
        g_variant_builder_clear(&children);
        unavailable();
      }
      g_variant_builder_add_value(
          &children, request.host->reference(child, request.sender));
    }
    return g_variant_new("(@a(so))", g_variant_builder_end(&children));
  }
  if (!strcmp(method, "GetIndexInParent"))
    return g_variant_new("(i)", element == request.host->root
                                    ? 0
                                    : integer(element->indexInParent()));
  if (!strcmp(method, "GetRelationSet"))
    return g_variant_new("(@a(ua(so)))", relations(request));
  if (!strcmp(method, "GetRole"))
    return g_variant_new("(u)", role(state));
  if (!strcmp(method, "GetRoleName") || !strcmp(method, "GetLocalizedRoleName"))
    return g_variant_new("(s)", roleName(state));
  if (!strcmp(method, "GetState"))
    return g_variant_new("(@au)", states(element, state));
  if (!strcmp(method, "GetAttributes"))
    return g_variant_new("(@a{ss})", attributes(objectAttributes(state)));
  if (!strcmp(method, "GetApplication"))
    return g_variant_new("(@(so))", request.host->application());
  if (!strcmp(method, "GetInterfaces"))
    return g_variant_new("(@as)", strings(interfaces(element, state)));
  unsupported();
}
} // namespace swax::atspi
