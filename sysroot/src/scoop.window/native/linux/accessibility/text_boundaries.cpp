#include "provider.h"
#include <algorithm>

namespace swax::atspi {
static std::pair<int64_t, int64_t> lineRange(const State &state, int64_t offset,
                                             bool ends, int direction) {
  if (!state.textGeometry)
    unavailable();
  const auto &text = publicContent(state);
  const auto &layout = state.textGeometry->data;
  auto index = layout.line_index(layout.context, offset, 1);
  if (index < 0)
    unavailable();
  auto end = [&](int64_t line) -> int64_t {
    const auto value = layout.line_range(layout.context, line);
    if (value.status == 0)
      return static_cast<int64_t>(text.size());
    if (value.status != 1)
      unavailable();
    auto result = value.end;
    while (result > value.start) {
      const char *previous =
          g_utf8_find_prev_char(text.data(), text.data() + result);
      auto ch = g_utf8_get_char(previous);
      if (ch != '\r' && ch != '\n' && ch != 0x2028 && ch != 0x2029)
        break;
      result = previous - text.data();
    }
    return result;
  };
  if (ends && offset >= end(index) &&
      offset < static_cast<int64_t>(text.size()))
    ++index;
  index += direction;
  if (index < 0)
    return {0, 0};
  const auto value = layout.line_range(layout.context, index);
  if (value.status == 0)
    return {text.size(), text.size()};
  if (value.status != 1)
    unavailable();
  return ends ? std::pair<int64_t, int64_t>{index ? end(index - 1) : 0,
                                            end(index)}
              : std::pair<int64_t, int64_t>{value.start, value.end};
}
GVariant *textAt(Request &request, int offset, uint32_t boundary, bool legacy,
                 int direction) {
  const auto &state = *request.state;
  const auto &text = publicContent(state);
  const auto position = textOffset(state, offset, true);
  int64_t first, last;
  if ((!legacy && boundary == 3) ||
      (legacy && (boundary == 5 || boundary == 6))) {
    auto range = lineRange(state, position, legacy && boundary == 6, direction);
    first = range.first;
    last = range.second;
  } else {
    if ((!legacy && boundary > 4) || (legacy && boundary > 6))
      invalid("Unknown text boundary");
    const auto count = integer(textUnits(text, text.size(), false));
    if (text.size() > INT_MAX || count == INT_MAX)
      unsupported("Text exceeds Pango boundary index range");
    std::vector<PangoLogAttr> attrs(static_cast<size_t>(count) + 1);
    pango_get_log_attrs(text.data(), static_cast<int>(text.size()), -1,
                        pango_language_get_default(), attrs.data(), count + 1);
    std::vector<int64_t> boundaries{0};
    const char *cursor = text.data();
    for (int index = 1; index <= count; ++index) {
      cursor = g_utf8_next_char(cursor);
      const auto &value = attrs[index];
      bool selected = boundary == 0;
      if (legacy) {
        if (boundary == 1)
          selected = value.is_word_start;
        if (boundary == 2)
          selected = value.is_word_end;
        if (boundary == 3)
          selected = value.is_sentence_start;
        if (boundary == 4)
          selected = value.is_sentence_end;
      } else {
        if (boundary == 1)
          selected = value.is_word_start || value.is_word_end;
        if (boundary == 2)
          selected = value.is_sentence_start || value.is_sentence_end;
        if (boundary == 4)
          selected = value.is_mandatory_break;
      }
      if (selected || index == count)
        boundaries.push_back(cursor - text.data());
    }
    if (!count)
      boundaries.push_back(0);
    auto found =
        std::upper_bound(boundaries.begin(), boundaries.end(), position);
    auto index = found - boundaries.begin() - 1;
    if (index >= static_cast<ptrdiff_t>(boundaries.size()) - 1)
      index = static_cast<ptrdiff_t>(boundaries.size()) - 2;
    index += direction;
    if (index < 0)
      first = last = 0;
    else if (static_cast<size_t>(index + 1) >= boundaries.size())
      first = last = text.size();
    else {
      first = boundaries[index];
      last = boundaries[index + 1];
    }
  }
  auto value = text.substr(static_cast<size_t>(first),
                           static_cast<size_t>(last - first));
  return g_variant_new("(@sii)", string(value),
                       integer(textUnits(text, first, false)),
                       integer(textUnits(text, last, false)));
}
} // namespace swax::atspi
