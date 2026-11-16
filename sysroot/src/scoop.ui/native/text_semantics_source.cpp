#include "text_semantics.h"
#include <memory>

UITextSource::UITextSource(UITextLayout value, const UITextSourceData &data)
    : layout(value), source_length(data.source_length),
      display_length(data.display_length), cells(data.cells != 0),
      expand(data.expand != 0), caret_width(data.caret_width),
      caret_aspect(data.caret_aspect),
      graphemes(data.graphemes, data.graphemes + data.grapheme_count),
      mapping(data.mapping_count
                  ? std::vector<UITextMapping>(
                        data.mapping, data.mapping + data.mapping_count)
                  : std::vector<UITextMapping>()),
      lines(data.line_count ? std::vector<UITextLine>(
                                  data.lines, data.lines + data.line_count)
                            : std::vector<UITextLine>()),
      cell_values(data.cell_count ? std::vector<UITextCell>(data.cell_values,
                                                            data.cell_values +
                                                                data.cell_count)
                                  : std::vector<UITextCell>()) {
  layout.retain(layout.context);
}
UITextSource::~UITextSource() { layout.release(layout.context); }
void UITextSource::retain() {
  references.fetch_add(1, std::memory_order_relaxed);
}
void UITextSource::release() {
  if (references.fetch_sub(1, std::memory_order_acq_rel) == 1)
    delete this;
}

// Read the same prepared replacement intervals used by the editor/selectable
// host.
int64_t UITextSource::map(int64_t offset, int32_t affinity,
                          bool backwards) const {
  int64_t shift = 0;
  for (const auto &item : mapping) {
    int64_t start = backwards ? item.display_start : item.source_start;
    int64_t end = backwards ? item.display_end : item.source_end;
    int64_t target_start = backwards ? item.source_start : item.display_start;
    int64_t target_end = backwards ? item.source_end : item.display_end;
    if (offset < start)
      return offset + shift;
    if (offset == start) {
      if (end != start || affinity == 0)
        return target_start;
    } else if (offset < end)
      return affinity == 0 ? target_start : target_end;
    shift = target_end - end;
  }
  return offset + shift;
}

int64_t UITextSource::visual(int64_t offset, int32_t affinity) const {
  auto range = layout.range(layout.context, offset, affinity, 0);
  if (range.status != 1)
    return -1;
  if (offset == range.range.start || offset == range.range.end)
    return offset;
  return affinity == 0 ? range.range.start : range.range.end;
}

UITextRangeResult UITextSource::source_range(UITextRange value) const {
  if (value.start == value.end) {
    int64_t offset = map(value.start, 1, true);
    return {{offset, offset}, 1};
  }
  return {{map(value.start, 0, true), map(value.end, 1, true)}, 1};
}

UITextRangeResult UITextSource::unit(int64_t offset, int32_t affinity,
                                     int32_t unit) const {
  if (offset < 0 || offset > source_length ||
      (affinity != 0 && affinity != 1) || unit < 0 || unit > 3)
    return {{}, -1};
  if (unit == 0) {
    if (source_length == 0 || (offset == source_length && affinity == 1))
      return {{offset, offset}, 1};
    auto end = std::upper_bound(graphemes.begin(), graphemes.end(), offset);
    if (affinity == 0 && offset > 0 && *(end - 1) == offset)
      --end;
    return {{*(end - 1), *end}, 1};
  }
  int64_t displayed = visual(map(offset, affinity, false), affinity);
  if (displayed < 0)
    return {{}, 0};
  if (cells && (unit == 2 || unit == 3))
    return source_range({0, display_length});
  auto result = layout.range(layout.context, displayed, affinity, unit);
  return result.status == 1 ? source_range(result.range) : result;
}

UITextRangeResult UITextSource::line_range(int64_t index) const {
  if (index < 0 || (uint64_t)index >= lines.size())
    return {{}, -1};
  const auto &line = lines[(size_t)index];
  return source_range({line.start, line.end});
}

int64_t UITextSource::line_index(int64_t offset, int32_t affinity) const {
  if (offset < 0 || offset > source_length || (affinity != 0 && affinity != 1))
    return -2;
  int64_t displayed = visual(map(offset, affinity, false), affinity);
  if (displayed < 0)
    return -1;
  return cells ? 0 : layout.line_index(layout.context, displayed, affinity);
}

UITextPosition UITextSource::hit(UITextPoint point) const {
  if (!cells)
    return layout.hit(layout.context, point);
  if (cell_values.empty())
    return {0, 1, 0};
  const UITextCell *nearest = &cell_values.front();
  double distance = std::abs(point.x - nearest->hit_x);
  for (const auto &cell : cell_values) {
    double next = std::abs(point.x - cell.hit_x);
    if (next < distance) {
      nearest = &cell;
      distance = next;
    }
  }
  return {nearest->start, 1, 1};
}

UITextCaret UITextSource::caret(int64_t offset, int32_t affinity) const {
  offset = visual(offset, affinity);
  if (offset < 0)
    return {{}, 0};
  if (cells) {
    for (const auto &cell : cell_values)
      if (cell.start == offset)
        return {cell.caret, 1};
    return {{}, 0};
  }
  if (caret_aspect == 0)
    return layout.caret(layout.context, offset, affinity, caret_width);
  auto measured = layout.caret(layout.context, offset, affinity, 0);
  return measured.status == 1
             ? layout.caret(layout.context, offset, affinity,
                            measured.rect.height * caret_aspect)
             : measured;
}

int32_t UITextSource::boxes(int64_t start, int64_t end, void *context,
                            UITextBoxVisitor visitor) const {
  if (!cells)
    return layout.boxes(layout.context, start, end, context, visitor);
  for (const auto &cell : cell_values)
    if (cell.start < cell.end && cell.start < end && cell.end > start)
      visitor(context, cell.bounds, 1);
  return 1;
}

extern "C" void *scoop_ui_semantic_text_source(UITextLayout layout,
                                               UITextSourceData data) {
  try {
    if (data.grapheme_count < 1 || data.mapping_count < 0 ||
        data.line_count < 0 || data.cell_count < 0)
      return nullptr;
    return new UITextSource(layout, data);
  } catch (...) {
    return nullptr;
  }
}

extern "C" void scoop_ui_semantic_text_source_release(UITextSource *source) {
  source->release();
}
