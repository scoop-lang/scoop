#include "provider.h"
#include <algorithm>

namespace swax::atspi {
GVariant *textAt(Request &request, int offset, uint32_t boundary, bool legacy,
                 int direction);
GVariant *textExtents(Request &request, int64_t start, int64_t end,
                      uint32_t coordinates);
GVariant *boundedText(Request &request, GVariant *parameters);
static std::pair<int64_t, int64_t> range(const State &state, int first,
                                         int last) {
  const auto start = textOffset(state, first, true);
  const auto end = last == -1
                       ? static_cast<int64_t>(publicContent(state).size())
                       : textOffset(state, last, true);
  if (end < start)
    invalid("Text range is reversed");
  return {start, end};
}
GVariant *text(Request &request, const char *method, GVariant *parameters) {
  const auto &state = *request.state;
  const auto &content = publicContent(state);
  const auto &data = state.data;
  const bool selection =
      data.anchor >= 0 && data.cursor >= 0 && data.anchor != data.cursor;
  if (!strcmp(method, "GetText")) {
    int first, last;
    g_variant_get(parameters, "(ii)", &first, &last);
    const auto bytes = range(state, first, last);
    return g_variant_new("(@s)", string(content.substr(
                                     bytes.first, bytes.second - bytes.first)));
  }
  if (!strcmp(method, "GetStringAtOffset") ||
      !strcmp(method, "GetTextAtOffset") ||
      !strcmp(method, "GetTextBeforeOffset") ||
      !strcmp(method, "GetTextAfterOffset")) {
    int offset;
    uint32_t boundary;
    g_variant_get(parameters, "(iu)", &offset, &boundary);
    return textAt(request, offset, boundary,
                  strcmp(method, "GetStringAtOffset") != 0,
                  !strcmp(method, "GetTextBeforeOffset")  ? -1
                  : !strcmp(method, "GetTextAfterOffset") ? 1
                                                          : 0);
  }
  if (!strcmp(method, "GetCharacterAtOffset")) {
    int offset;
    g_variant_get(parameters, "(i)", &offset);
    auto byte = textOffset(state, offset);
    return g_variant_new(
        "(i)", static_cast<int>(g_utf8_get_char(content.data() + byte)));
  }
  if (!strcmp(method, "SetCaretOffset")) {
    int offset;
    g_variant_get(parameters, "(i)", &offset);
    auto byte = textOffset(state, offset, true);
    return g_variant_new("(b)", actionResult(request.element->perform(
                                    5, {}, byte, byte, data.content_revision)));
  }
  if (!strcmp(method, "GetNSelections"))
    return g_variant_new("(i)", selection ? 1 : 0);
  if (!strcmp(method, "GetSelection") || !strcmp(method, "RemoveSelection")) {
    int index;
    g_variant_get(parameters, "(i)", &index);
    if (index != 0 || !selection)
      invalid("Selection index is out of range");
    if (!strcmp(method, "GetSelection"))
      return g_variant_new(
          "(ii)",
          integer(
              textUnits(content, std::min(data.anchor, data.cursor), false)),
          integer(
              textUnits(content, std::max(data.anchor, data.cursor), false)));
    return g_variant_new(
        "(b)", actionResult(request.element->perform(
                   5, {}, data.cursor, data.cursor, data.content_revision)));
  }
  if (!strcmp(method, "SetSelection") || !strcmp(method, "AddSelection")) {
    int index = 0, first, last;
    if (!strcmp(method, "SetSelection"))
      g_variant_get(parameters, "(iii)", &index, &first, &last);
    else
      g_variant_get(parameters, "(ii)", &first, &last);
    if (index != 0)
      invalid("Only a single text selection is supported");
    const auto bytes = range(state, first, last);
    if (!strcmp(method, "AddSelection") && selection &&
        (std::min(data.anchor, data.cursor) != bytes.first ||
         std::max(data.anchor, data.cursor) != bytes.second))
      return g_variant_new("(b)", FALSE);
    return g_variant_new(
        "(b)", actionResult(request.element->perform(
                   5, {}, bytes.first, bytes.second, data.content_revision)));
  }
  if (!strcmp(method, "GetDefaultAttributes") ||
      !strcmp(method, "GetDefaultAttributeSet"))
    return g_variant_new("(@a{ss})", textAttributes(state));
  if (!strcmp(method, "GetAttributeValue")) {
    int offset;
    const char *name;
    g_variant_get(parameters, "(i&s)", &offset, &name);
    textOffset(state, offset, true);
    const auto value =
        !strcmp(name, "editable")
            ? (!(data.flags & SWAXReadOnly) && (data.actions & (4 | 32))
                   ? "true"
                   : "false")
            : "";
    return g_variant_new("(s)", value);
  }
  if (!strcmp(method, "GetAttributes") || !strcmp(method, "GetAttributeRun")) {
    int offset;
    gboolean defaults = TRUE;
    if (!strcmp(method, "GetAttributeRun"))
      g_variant_get(parameters, "(ib)", &offset, &defaults);
    else
      g_variant_get(parameters, "(i)", &offset);
    textOffset(state, offset, true);
    return g_variant_new("(@a{ss}ii)",
                         defaults ? textAttributes(state) : attributes({}), 0,
                         integer(textUnits(content, content.size(), false)));
  }
  if (!strcmp(method, "GetCharacterExtents")) {
    int offset;
    uint32_t coordinates;
    g_variant_get(parameters, "(iu)", &offset, &coordinates);
    auto byte = textOffset(state, offset);
    auto end = g_utf8_next_char(content.data() + byte) - content.data();
    return textExtents(request, byte, end, coordinates);
  }
  if (!strcmp(method, "GetRangeExtents")) {
    int first, last;
    uint32_t coordinates;
    g_variant_get(parameters, "(iiu)", &first, &last, &coordinates);
    const auto bytes = range(state, first, last);
    return textExtents(request, bytes.first, bytes.second, coordinates);
  }
  if (!strcmp(method, "GetOffsetAtPoint")) {
    int x, y;
    uint32_t coordinates;
    g_variant_get(parameters, "(iiu)", &x, &y, &coordinates);
    auto point = localPoint(
        request, {static_cast<double>(x), static_cast<double>(y)}, coordinates);
    if (!state.textGeometry)
      unavailable();
    const auto &layout = state.textGeometry->data;
    auto offset = layout.range_at_point(layout.context, point.x, point.y);
    if (offset.status < 0)
      unavailable();
    return g_variant_new("(i)",
                         offset.status == 1
                             ? integer(textUnits(content, offset.start, false))
                             : -1);
  }
  if (!strcmp(method, "GetBoundedRanges"))
    return boundedText(request, parameters);
  if (!strcmp(method, "ScrollSubstringTo")) {
    int first, last;
    uint32_t placement;
    g_variant_get(parameters, "(iiu)", &first, &last, &placement);
    const auto bytes = range(state, first, last);
    static const double alignment[] = {0, 10, 3, 11, 12, 14, 15};
    if (placement >= G_N_ELEMENTS(alignment))
      invalid("Unknown scroll placement");
    return g_variant_new("(b)",
                         actionResult(request.element->perform(
                             21, {}, bytes.first, bytes.second,
                             data.content_revision, alignment[placement])));
  }
  if (!strcmp(method, "ScrollSubstringToPoint"))
    unsupported("Text supports edge and nearest alignment; arbitrary screen "
                "placement is unavailable");
  unsupported();
}
} // namespace swax::atspi
