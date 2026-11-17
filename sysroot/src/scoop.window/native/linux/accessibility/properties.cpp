#include "provider.h"
#include <algorithm>
#include <cmath>

namespace swax::atspi {
GVariant *property(Request &request, const char *interface, const char *name) {
  const auto &state = *request.state;
  const auto &data = state.data;
  if (!strcmp(name, "version"))
    return g_variant_new_uint32(1);
  if (!strcmp(interface, "org.a11y.atspi.Accessible")) {
    if (!strcmp(name, "Name"))
      return string(state.text[0].value_or(""));
    if (!strcmp(name, "Description"))
      return string(state.text[1].value_or(""));
    if (!strcmp(name, "HelpText"))
      return string(state.text[4].value_or(state.text[1].value_or("")));
    if (!strcmp(name, "AccessibleId"))
      return string(state.text[2].value_or(""));
    if (!strcmp(name, "Locale"))
      return g_variant_new_string(g_get_language_names()[0]);
    if (!strcmp(name, "ChildCount"))
      return g_variant_new_int32(integer(request.element->childCount()));
    if (!strcmp(name, "Parent"))
      return request.element == request.host->root
                 ? request.host->parent()
                 : request.host->reference(request.element->parent(),
                                           request.sender);
  } else if (!strcmp(interface, "org.a11y.atspi.Action")) {
    if (!strcmp(name, "NActions"))
      return g_variant_new_int32(integer(actions(state).size()));
  } else if (!strcmp(interface, "org.a11y.atspi.Value")) {
    const bool scroll = data.role == 6;
    const double maximum =
        scroll ? std::max(0.0, data.scroll_content - data.scroll_viewport)
               : data.maximum;
    if (!strcmp(name, "MinimumValue"))
      return g_variant_new_double(scroll ? 0 : data.minimum);
    if (!strcmp(name, "MaximumValue"))
      return g_variant_new_double(maximum);
    if (!strcmp(name, "CurrentValue"))
      return g_variant_new_double(scroll ? data.scroll_offset
                                         : data.number_value);
    if (!strcmp(name, "MinimumIncrement"))
      return g_variant_new_double(data.number_step);
    if (!strcmp(name, "Text"))
      return string(state.text[3].value_or(""));
  } else if (!strcmp(interface, "org.a11y.atspi.Selection")) {
    if (!strcmp(name, "NSelectedChildren"))
      return g_variant_new_int32(integer(request.element->selection().size()));
  } else if (!strcmp(interface, "org.a11y.atspi.Text")) {
    const auto &content = publicContent(state);
    if (!strcmp(name, "CharacterCount"))
      return g_variant_new_int32(
          integer(textUnits(content, content.size(), false)));
    if (!strcmp(name, "CaretOffset"))
      return g_variant_new_int32(
          data.cursor < 0 ? -1
                          : integer(textUnits(content, data.cursor, false)));
  } else if (!strcmp(interface, "org.a11y.atspi.Table")) {
    if (!strcmp(name, "NRows"))
      return g_variant_new_int32(integer(request.element->rowCount()));
    if (!strcmp(name, "NColumns"))
      return g_variant_new_int32(integer(request.element->columnCount()));
    if (!strcmp(name, "Caption")) {
      auto owner = request.element->tree.lock();
      auto caption = state.relations[0].size() == 1
                         ? owner->actual(state.relations[0][0])
                         : nullptr;
      return request.host->reference(
          caption && caption->exposed() ? caption : nullptr, request.sender);
    }
    if (!strcmp(name, "Summary")) {
      auto owner = request.element->tree.lock();
      auto summary = state.relations[3].size() == 1
                         ? owner->actual(state.relations[3][0])
                         : nullptr;
      return request.host->reference(
          summary && summary->exposed() ? summary : nullptr, request.sender);
    }
    if (!strcmp(name, "NSelectedRows"))
      return g_variant_new_int32(integer(selectedRows(request.element).size()));
    if (!strcmp(name, "NSelectedColumns")) {
      auto count = request.element->rowCount();
      return g_variant_new_int32(
          count > 0 && static_cast<int64_t>(
                           selectedRows(request.element).size()) == count
              ? integer(request.element->columnCount())
              : 0);
    }
  } else if (!strcmp(interface, "org.a11y.atspi.TableCell")) {
    if (!strcmp(name, "RowSpan"))
      return g_variant_new_int32(integer(data.row_span));
    if (!strcmp(name, "ColumnSpan"))
      return g_variant_new_int32(integer(data.column_span));
    if (!strcmp(name, "Position"))
      return g_variant_new("(ii)", integer(data.cell_row),
                           integer(data.cell_column));
    if (!strcmp(name, "Table"))
      return request.host->reference(containingTable(request.element),
                                     request.sender);
  } else if (!strcmp(interface, "org.a11y.atspi.Hyperlink")) {
    if (!strcmp(name, "NAnchors"))
      return g_variant_new_int16(1);
    std::shared_ptr<const State> parent;
    const auto child = textChild(request.element, parent);
    if (!strcmp(name, "StartIndex"))
      return g_variant_new_int32(
          child
              ? integer(textUnits(publicContent(*parent), child->start, false))
              : -1);
    if (!strcmp(name, "EndIndex"))
      return g_variant_new_int32(
          child ? integer(textUnits(publicContent(*parent), child->end, false))
                : -1);
  }
  unsupported("The element does not provide this property");
}
void setProperty(Request &request, const char *interface, const char *name,
                 GVariant *value) {
  if (strcmp(interface, "org.a11y.atspi.Value") || strcmp(name, "CurrentValue"))
    unsupported("This property is read-only");
  if (!g_variant_is_of_type(value, G_VARIANT_TYPE_DOUBLE))
    invalid();
  const double position = g_variant_get_double(value);
  const auto &data = request.state->data;
  if (!std::isfinite(position))
    invalid();
  int32_t status;
  if (data.role == 6) {
    const auto maximum =
        std::max(0.0, data.scroll_content - data.scroll_viewport);
    if (position < 0 || position > maximum)
      invalid();
    status = request.element->perform(8, {}, -1, -1, 0,
                                      maximum ? position / maximum : 0);
  } else
    status = request.element->perform(12, {}, -1, -1, 0, position);
  if (!actionResult(status))
    throw Failure("org.a11y.atspi.Error.Failed",
                  "The control rejected the value");
}
} // namespace swax::atspi
