#include "text.h"

namespace {
bool affinity_valid(int32_t affinity) { return affinity == 0 || affinity == 1; }

bool scalar_finite(double value) {
  return std::isfinite(value) && std::isfinite((float)value);
}

void retain(void *context) { static_cast<STTextLayout *>(context)->retain(); }
void release(void *context) { static_cast<STTextLayout *>(context)->release(); }

STTextHit hit(void *context, STPoint point) {
  if (!scalar_finite(point.x) || !scalar_finite(point.y))
    return {0, 1, -1};
  auto result = scoop_graphics_text_hit(context, point);
  if (result.status == 0)
    result.status = -2;
  return result;
}

STCaret caret(void *context, int64_t byte, int32_t affinity, double thickness) {
  if (!affinity_valid(affinity) || !scalar_finite(thickness) || thickness < 0)
    return {{}, -1};
  return scoop_graphics_text_caret(context, byte, affinity, thickness);
}

using BoxVisitor = void (*)(void *, STRect, int32_t);

int32_t boxes(void *context, int64_t start, int64_t end, void *visitor_context,
              BoxVisitor visitor) {
  if (!visitor)
    return -1;
  try {
    const auto result = scoop_graphics_text_selection(context, start, end);
    if (result.status != 1)
      return result.status == 0 ? -2 : result.status;
    const std::unique_ptr<STTextBoxes> owned(
        static_cast<STTextBoxes *>(result.pointer));
    for (const auto &box : owned->values)
      visitor(visitor_context, st_rect(box.rect),
              box.direction == sgt::TextDirection::kLtr ? 1 : 0);
    return 1;
  } catch (...) {
    return -2;
  }
}

STRangeQuery range(void *context, int64_t byte, int32_t affinity,
                   int32_t unit) {
  if (!affinity_valid(affinity) || unit < 0 || unit > 3)
    return {{}, -1};
  try {
    auto *layout = static_cast<STTextLayout *>(context);
    auto &data = *layout->data;
    const std::lock_guard<std::recursive_mutex> guard(data.mutex);
    if (!data.scalar(byte))
      return {{}, -1};
    if (unit == 1)
      return scoop_graphics_text_word_range(context, byte, affinity);
    if (unit == 2) {
      const int index = layout->line_at((size_t)byte, affinity == 0);
      return index < 0 ? STRangeQuery{{}, 0}
                       : STRangeQuery{layout->lines[(size_t)index].range, 1};
    }
    if (unit == 3)
      return scoop_graphics_text_logical_line(context, byte, affinity);
    if (data.text.empty() ||
        ((size_t)byte == data.text.size() && affinity == 1))
      return {{byte, byte}, 1};
    auto end =
        std::upper_bound(data.graphemes.begin(), data.graphemes.end(), byte);
    if (affinity == 0 && byte > 0 && *(end - 1) == byte)
      --end;
    return {{*(end - 1), *end}, 1};
  } catch (...) {
    return {{}, -2};
  }
}

STRangeQuery line_range(void *context, int64_t index) {
  const auto *layout = static_cast<const STTextLayout *>(context);
  if (index < 0 || (uint64_t)index >= layout->lines.size())
    return {{}, -1};
  return {layout->lines[(size_t)index].range, 1};
}

int64_t line_index(void *context, int64_t byte, int32_t affinity) {
  const auto *layout = static_cast<const STTextLayout *>(context);
  if (!affinity_valid(affinity) || !layout->data->scalar(byte))
    return -2;
  return layout->line_at((size_t)byte, affinity == 0);
}
} // namespace

struct STNativeTextLayout {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  STTextHit (*hit)(void *, STPoint);
  STCaret (*caret)(void *, int64_t, int32_t, double);
  int32_t (*boxes)(void *, int64_t, int64_t, void *, BoxVisitor);
  STRangeQuery (*range)(void *, int64_t, int32_t, int32_t);
  STRangeQuery (*line_range)(void *, int64_t);
  int64_t (*line_index)(void *, int64_t, int32_t);
};

extern "C" STNativeTextLayout scoop_graphics_text_export_layout(void *context) {
  retain(context);
  return {context, retain, release,    hit,       caret,
          boxes,   range,  line_range, line_index};
}
