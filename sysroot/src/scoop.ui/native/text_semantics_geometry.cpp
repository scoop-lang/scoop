#include "text_semantics.h"

namespace {
struct Geometry {
  std::atomic<uint64_t> references{1};
  UITextSource *source;
  UITextSceneGeometry scene;
  Geometry(UITextSource *source, UITextSceneGeometry scene)
      : source(source), scene(scene) {
    source->retain();
    scene.retain(scene.context);
  }
  ~Geometry() {
    scene.release(scene.context);
    source->release();
  }
};

void retain(void *context) {
  static_cast<Geometry *>(context)->references.fetch_add(
      1, std::memory_order_relaxed);
}
void release(void *context) {
  auto *value = static_cast<Geometry *>(context);
  if (value->references.fetch_sub(1, std::memory_order_acq_rel) == 1)
    delete value;
}
UITextRangeResult range(void *context, int64_t offset, int32_t affinity,
                        int32_t unit) {
  return static_cast<Geometry *>(context)->source->unit(offset, affinity, unit);
}
UITextRangeResult line_range(void *context, int64_t index) {
  return static_cast<Geometry *>(context)->source->line_range(index);
}
int64_t line_index(void *context, int64_t offset, int32_t affinity) {
  return static_cast<Geometry *>(context)->source->line_index(offset, affinity);
}
UITextRangeResult at_point(void *context, double x, double y) {
  const auto &value = *static_cast<Geometry *>(context);
  if (!std::isfinite(x) || !std::isfinite(y))
    return {{}, -1};
  if (!value.scene.contains(value.scene.context, x, y))
    return {{}, 0};
  auto local = value.scene.to_local(value.scene.context, x, y);
  if (local.status != 1)
    return {{}, local.status};
  auto hit = value.source->hit(local.point);
  if (hit.status != 1)
    return {{}, hit.status};
  auto selected = value.source->layout.range(value.source->layout.context,
                                             hit.byte, hit.affinity, 0);
  return selected.status == 1 ? value.source->source_range(selected.range)
                              : selected;
}

struct BoxMapping {
  const Geometry *geometry;
  void *context;
  UISemanticBoxVisitor visitor;
  int32_t status = 1;
  bool any = false;
};
void emit_box(void *context, UITextRect rect, int32_t direction) {
  auto &output = *static_cast<BoxMapping *>(context);
  const auto &scene = output.geometry->scene;
  auto mapped = scene.map_rect(scene.context, rect, 0);
  if (mapped.status != 1) {
    output.status = mapped.status;
    return;
  }
  output.any = true;
  const auto &box = mapped.rect;
  output.visitor(output.context, box.x, box.y, box.width, box.height);
}
int32_t boxes(void *context, int64_t start, int64_t end, void *target,
              UISemanticBoxVisitor visitor) {
  const auto &value = *static_cast<Geometry *>(context);
  const auto &source = *value.source;
  if (start < 0 || end < start || end > source.source_length || !visitor)
    return -1;
  BoxMapping output{&value, target, visitor};
  if (start == end) {
    auto caret = source.caret(source.map(start, 1, false), 1);
    if (caret.status != 1)
      return caret.status;
    emit_box(&output, caret.rect, 1);
  } else {
    int64_t from = source.map(start, source.expand ? 0 : 1, false);
    int64_t to = source.map(end, source.expand ? 1 : 0, false);
    if (to <= from)
      return 0;
    int32_t status = source.boxes(from, to, &output, emit_box);
    if (status != 1)
      return status;
  }
  return output.status != 1 ? output.status : output.any ? 1 : 0;
}

struct Visibility {
  const Geometry *geometry;
  bool visible = false;
  bool has_boxes = false;
  int32_t status = 1;
};
void visible_box(void *context, UITextRect rect, int32_t direction) {
  auto &result = *static_cast<Visibility *>(context);
  result.has_boxes = true;
  const auto &scene = result.geometry->scene;
  auto mapped = scene.map_rect(scene.context, rect, 1);
  if (mapped.status < 0)
    result.status = mapped.status;
  result.visible |= mapped.status == 1;
}
int32_t visible(const Geometry &geometry, int64_t start, int64_t end) {
  Visibility result{&geometry};
  if (start != end) {
    int32_t status = geometry.source->boxes(start, end, &result, visible_box);
    if (status != 1)
      return status;
  }
  if (!result.has_boxes && result.status == 1) {
    auto caret = geometry.source->caret(start, 1);
    if (caret.status < 0)
      return caret.status;
    if (caret.status == 1)
      visible_box(&result, caret.rect, 1);
  }
  return result.status < 0 ? result.status : result.visible ? 1 : 0;
}
UITextRangeResult visible_range(void *context) {
  const auto &value = *static_cast<Geometry *>(context);
  const auto &source = *value.source;
  UITextRangeResult result{{}, 0};
  for (const auto &line : source.lines) {
    int32_t status = visible(value, line.visible_start, line.visible_end);
    if (status < 0)
      return {{}, status};
    if (status == 0)
      continue;
    int64_t offset = line.visible_start;
    do {
      auto unit = source.layout.range(source.layout.context, offset, 1, 0);
      if (unit.status != 1)
        return {{}, unit.status};
      unit.range.end = std::min(unit.range.end, line.visible_end);
      status = visible(value, unit.range.start, unit.range.end);
      if (status < 0)
        return {{}, status};
      if (status == 1) {
        auto mapped = source.source_range(unit.range);
        if (result.status == 0)
          result = mapped;
        else {
          result.range.start = std::min(result.range.start, mapped.range.start);
          result.range.end = std::max(result.range.end, mapped.range.end);
        }
      }
      if (unit.range.end <= offset)
        break;
      offset = unit.range.end;
    } while (offset < line.visible_end);
  }
  return result;
}
} // namespace

extern "C" UISemanticTextGeometry
scoop_ui_semantic_text_geometry(UITextSource *source,
                                UITextSceneGeometry scene) {
  try {
    return {new Geometry(source, scene),
            retain,
            release,
            range,
            line_range,
            line_index,
            at_point,
            boxes,
            visible_range};
  } catch (...) {
    return {nullptr,    retain,   release, range,        line_range,
            line_index, at_point, boxes,   visible_range};
  }
}
