#include <algorithm>
#include <cstdint>
#include <thread>

namespace {
struct Point {
  double x, y;
};
struct Rect {
  double x, y, width, height;
};
struct Range {
  int64_t start, end;
};
struct Position {
  int64_t byte;
  int32_t affinity, status;
};
struct Caret {
  Rect rect;
  int32_t status;
};
struct RangeQuery {
  Range range;
  int32_t status;
};
using Visitor = void (*)(void *, Rect, int32_t);

struct Layout {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  Position (*hit)(void *, Point);
  Caret (*caret)(void *, int64_t, int32_t, double);
  int32_t (*boxes)(void *, int64_t, int64_t, void *, Visitor);
  RangeQuery (*range)(void *, int64_t, int32_t, int32_t);
  RangeQuery (*line_range)(void *, int64_t);
  int64_t (*line_index)(void *, int64_t, int32_t);
};

struct Boxes {
  int64_t count, rtl, ltr;
  Rect bounds;
  int32_t status;
};

void collect(void *context, Rect rect, int32_t direction) {
  auto &value = *static_cast<Boxes *>(context);
  if (value.count == 0)
    value.bounds = rect;
  else {
    const double left = std::min(value.bounds.x, rect.x);
    const double top = std::min(value.bounds.y, rect.y);
    const double right =
        std::max(value.bounds.x + value.bounds.width, rect.x + rect.width);
    const double bottom =
        std::max(value.bounds.y + value.bounds.height, rect.y + rect.height);
    value.bounds = {left, top, right - left, bottom - top};
  }
  ++value.count;
  if (direction == 0)
    ++value.rtl;
  if (direction == 1)
    ++value.ltr;
}

Boxes read(Layout layout, Range range) {
  Boxes result{};
  result.status =
      layout.boxes(layout.context, range.start, range.end, &result, collect);
  return result;
}

bool same(const Boxes &a, const Boxes &b) {
  return a.count == b.count && a.rtl == b.rtl && a.ltr == b.ltr &&
         a.bounds.x == b.bounds.x && a.bounds.y == b.bounds.y &&
         a.bounds.width == b.bounds.width &&
         a.bounds.height == b.bounds.height && a.status == b.status;
}
} // namespace

// The visitor and the foreign worker stay entirely in native code.
extern "C" Boxes scoop_graphics_test_text_boxes(Layout layout, Range range) {
  std::thread worker;
  Boxes remote{};
  layout.retain(layout.context);
  try {
    worker = std::thread([&] {
      remote = read(layout, range);
      layout.release(layout.context);
    });
  } catch (...) {
    layout.release(layout.context);
    return {0, 0, 0, {}, -2};
  }
  const Boxes local = read(layout, range);
  worker.join();
  if (!same(local, remote))
    remote.status = -2;
  return remote;
}
