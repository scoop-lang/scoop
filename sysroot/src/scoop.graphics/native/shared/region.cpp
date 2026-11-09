#include "graphics.h"
#include "include/core/SkRegion.h"
#include <limits>

struct SGRegion : SGObject {
  std::vector<SGRect> rectangles;
};

static std::optional<SkIRect> grid_bounds(SkRect rect, double scale) {
  double left = floor(rect.left() * scale), top = floor(rect.top() * scale);
  double right = ceil(rect.right() * scale),
         bottom = ceil(rect.bottom() * scale);
  if (!std::isfinite(scale) || scale <= 0 || !rect.isFinite() ||
      left < INT32_MIN || top < INT32_MIN || right > INT32_MAX ||
      bottom > INT32_MAX || right - left > INT32_MAX ||
      bottom - top > INT32_MAX)
    return std::nullopt;
  return SkIRect::MakeLTRB((int32_t)left, (int32_t)top, (int32_t)right,
                           (int32_t)bottom);
}

static bool raster(SkRegion &output, const SkPath &path, double scale,
                   const SkRegion &clip) {
  auto transformed =
      path.tryMakeTransform(SkMatrix::Scale((float)scale, (float)scale));
  if (!transformed || !transformed->isFinite())
    return false;
  output.setPath(*transformed, clip);
  return true;
}

static SGRegion *rectangles(const SkRegion &region, double scale) {
  auto result = std::make_unique<SGRegion>();
  for (SkRegion::Iterator iterator(region); !iterator.done(); iterator.next()) {
    const SkIRect &rect = iterator.rect();
    result->rectangles.push_back({rect.left() / scale, rect.top() / scale,
                                  rect.width() / scale, rect.height() / scale});
  }
  return result.release();
}

extern "C" {
SGRegion *scoop_graphics_path_region(SGPath *value, SGRect clip, double scale) {
  try {
    auto bounds = grid_bounds(sg_rect(clip), scale);
    if (!bounds)
      return nullptr;
    SkRegion region;
    if (!raster(region, value->path, scale, SkRegion(*bounds)))
      return nullptr;
    return rectangles(region, scale);
  } catch (...) {
    return nullptr;
  }
}

SGRegion *scoop_graphics_geometry_region(SGGeometry *value, double scale) {
  try {
    auto bounds = grid_bounds(value->viewport, scale);
    if (!bounds)
      return nullptr;
    if (!value->invertible)
      return new SGRegion();
    auto path = value->shape.tryMakeTransform(value->matrix);
    if (!path)
      return nullptr;
    SkRegion region;
    const SkRegion viewport(*bounds);
    if (!raster(region, *path, scale, viewport))
      return nullptr;
    for (auto clip = value->clips.get(); clip; clip = clip->previous.get()) {
      SkRegion part;
      if (!raster(part, clip->path, scale, viewport))
        return nullptr;
      region.op(part, SkRegion::kIntersect_Op);
    }
    return rectangles(region, scale);
  } catch (...) {
    return nullptr;
  }
}

int64_t scoop_graphics_region_count(SGRegion *value) {
  return (int64_t)value->rectangles.size();
}
SGRect scoop_graphics_region_rect(SGRegion *value, int64_t index) {
  return value->rectangles[(size_t)index];
}
}
