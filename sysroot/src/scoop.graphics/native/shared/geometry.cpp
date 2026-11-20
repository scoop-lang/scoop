#include "graphics.h"
#include "include/pathops/SkPathOps.h"

static SGPoint queryPoint(const SkMatrix &matrix, SGPoint point) {
  if (matrix.hasPerspective())
    return sg_point(matrix.mapPoint(sg_point(point)));
  return {static_cast<double>(matrix.getScaleX()) * point.x +
              static_cast<double>(matrix.getSkewX()) * point.y +
              matrix.getTranslateX(),
          static_cast<double>(matrix.getSkewY()) * point.x +
              static_cast<double>(matrix.getScaleY()) * point.y +
              matrix.getTranslateY()};
}

static SGRect queryRect(const SkMatrix &matrix, SGRect rect) {
  const auto mapped = sg_rect(matrix.mapRect(sg_rect(rect)));
  if (matrix.hasPerspective() || (mapped.width != 0 && mapped.height != 0))
    return mapped;
  // Distant translations can round both native edges to the same scalar.
  // Keep the captured affine transform and recover extents before translating.
  const auto origin = queryPoint(matrix, {rect.x, rect.y});
  const double xx = static_cast<double>(matrix.getScaleX()) * rect.width;
  const double xy = static_cast<double>(matrix.getSkewX()) * rect.height;
  const double yx = static_cast<double>(matrix.getSkewY()) * rect.width;
  const double yy = static_cast<double>(matrix.getScaleY()) * rect.height;
  return {origin.x + std::min(0.0, xx) + std::min(0.0, xy),
          origin.y + std::min(0.0, yx) + std::min(0.0, yy),
          std::abs(xx) + std::abs(xy), std::abs(yx) + std::abs(yy)};
}

static int32_t clip(SGCanvas *value, const SkPath &path) {
  try {
    auto transformed = path.tryMakeTransform(value->logical());
    if (!transformed)
      return -1;
    auto previous = value->clips.back();
    SkRect currentRect, previousRect;
    if (previous && transformed->isRect(&currentRect) &&
        previous->path.isRect(&previousRect)) {
      if (!currentRect.intersect(previousRect))
        currentRect.setEmpty();
      transformed = SkPathBuilder().addRect(currentRect).detach();
      previous = previous->previous;
    }
    auto next =
        std::make_shared<SGClip>(std::move(*transformed), std::move(previous));
    value->canvas->clipPath(path, true);
    value->clips.back() = std::move(next);
    return 1;
  } catch (...) {
    return 0;
  }
}
static SGGeometry *geometry(SGCanvas *value, SkPath shape) {
  try {
    auto result = std::make_unique<SGGeometry>();
    result->matrix = value->logical();
    result->invertible = result->matrix.invert(&result->inverse);
    result->clips = value->clips.back();
    result->shape = std::move(shape);
    result->viewport = value->viewport;
    return result.release();
  } catch (...) {
    return nullptr;
  }
}

extern "C" {
SGTransform scoop_graphics_canvas_pixel_transform(SGCanvas *value) {
  const auto matrix = value->canvas->getLocalToDeviceAs3x3();
  return {matrix.getScaleX(), matrix.getSkewY(),      matrix.getSkewX(),
          matrix.getScaleY(), matrix.getTranslateX(), matrix.getTranslateY()};
}
SGBounds scoop_graphics_canvas_clip_bounds(SGCanvas *value) {
  SkRect result;
  if (!value->canvas->getLocalClipBounds(&result))
    return {};
  return {1, sg_rect(result)};
}
int32_t scoop_graphics_canvas_save(SGCanvas *value, double opacity, bool layer,
                                   SGFilter *filter, int32_t blend,
                                   bool hasBounds, SGRect bounds) {
  SkBlendMode mode;
  switch (blend) {
  case 0:
    mode = SkBlendMode::kSrcOver;
    break;
  case 1:
    mode = SkBlendMode::kSrc;
    break;
  case 2:
    mode = SkBlendMode::kDstIn;
    break;
  case 3:
    mode = SkBlendMode::kPlus;
    break;
  case 4:
    mode = SkBlendMode::kDarken;
    break;
  case 5:
    mode = SkBlendMode::kLighten;
    break;
  default:
    return -1;
  }
  const size_t previousDepth = value->clips.size();
  try {
    value->clips.push_back(value->clips.back());
    if (layer) {
      SkPaint paint;
      paint.setAlphaf((float)opacity);
      paint.setBlendMode(mode);
      if (filter) {
        paint.setImageFilter(filter->filter);
        if (value->recording)
          value->recording->independentTiles = false;
      }
      if (!(opacity > 0 && opacity < 1 && !filter && !hasBounds &&
            mode == SkBlendMode::kSrcOver && value->saveOpacityLayer(paint))) {
        const SkRect sourceBounds = sg_rect(bounds);
        value->canvas->saveLayer(hasBounds ? &sourceBounds : nullptr, &paint);
      }
    } else {
      value->canvas->save();
    }
    return 1;
  } catch (...) {
    value->clips.resize(previousDepth);
    return 0;
  }
}
int32_t scoop_graphics_canvas_restore(SGCanvas *value) {
  if (value->clips.size() <= 1)
    return -1;
  value->restore();
  return 1;
}
int32_t scoop_graphics_canvas_concat(SGCanvas *value, SGTransform transform) {
  SkMatrix matrix = sg_matrix(transform);
  SkMatrix next =
      SkMatrix::Concat(value->canvas->getLocalToDeviceAs3x3(), matrix);
  if (!next.isFinite())
    return -1;
  value->canvas->concat(matrix);
  return 1;
}
int32_t scoop_graphics_canvas_clip_rect(SGCanvas *value, SGRect rect) {
  try {
    return clip(value, SkPathBuilder().addRect(sg_rect(rect)).detach());
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_canvas_clip_path(SGCanvas *value, SGPath *path) {
  return clip(value, path->path);
}
SGGeometry *scoop_graphics_geometry_rect(SGCanvas *value, SGRect rect) {
  try {
    return geometry(value, SkPathBuilder().addRect(sg_rect(rect)).detach());
  } catch (...) {
    return nullptr;
  }
}
SGGeometry *scoop_graphics_geometry_path(SGCanvas *value, SGPath *path) {
  return geometry(value, path->path);
}
int32_t scoop_graphics_geometry_hit(const SGGeometry *value, SGPoint point) {
  if (!value->invertible || !std::isfinite(point.x) || !std::isfinite(point.y))
    return 0;
  SkPoint window = sg_point(point);
  if (!value->viewport.contains(window.x(), window.y()))
    return 0;
  if (value->clips) {
    const auto bounds = value->clips->bounds;
    if (bounds.isEmpty() || window.x() < bounds.left() ||
        window.x() > bounds.right() || window.y() < bounds.top() ||
        window.y() > bounds.bottom())
      return 0;
  }
  for (auto clip = value->clips.get(); clip; clip = clip->previous.get())
    if (!clip->path.contains(window))
      return 0;
  return value->shape.contains(value->inverse.mapPoint(window));
}
SGSample scoop_graphics_geometry_point(const SGGeometry *value, SGPoint point,
                                       bool inverse) {
  if (inverse && !value->invertible)
    return {};
  const auto result =
      queryPoint(inverse ? value->inverse : value->matrix, point);
  if (!std::isfinite(result.x) || !std::isfinite(result.y))
    return {};
  return {1, result, {0, 0}};
}
SGRect scoop_graphics_geometry_rect_map(const SGGeometry *value, SGRect rect) {
  return queryRect(value->matrix, rect);
}
SGRect scoop_graphics_geometry_bounds(SGGeometry *value) {
  return queryRect(value->matrix, sg_rect(value->shape.computeTightBounds()));
}
SGBounds scoop_graphics_geometry_visible(SGGeometry *value) {
  if (value->visibilityKnown)
    return value->visible;
  try {
    if (!value->invertible) {
      value->visibilityKnown = true;
      return {};
    }
    auto path = value->shape.tryMakeTransform(value->matrix);
    if (!path)
      return {-1, {}};
    SkRect bounds;
    if (path->isRect(&bounds) &&
        (!value->clips ||
         (!value->clips->previous && value->clips->path.isRect(nullptr)))) {
      if (!bounds.intersect(value->viewport) ||
          (value->clips && !bounds.intersect(value->clips->bounds)))
        bounds.setEmpty();
      value->visible =
          bounds.isEmpty() ? SGBounds{} : SGBounds{1, sg_rect(bounds)};
      value->visibilityKnown = true;
      return value->visible;
    }
    for (auto clip = value->clips.get(); clip; clip = clip->previous.get()) {
      path = Op(*path, clip->path, kIntersect_SkPathOp);
      if (!path)
        return {-1, {}};
    }
    path = Op(*path, SkPathBuilder().addRect(value->viewport).detach(),
              kIntersect_SkPathOp);
    if (!path)
      return {-1, {}};
    bounds = path->computeTightBounds();
    if (!bounds.isFinite())
      return {-1, {}};
    value->visible =
        bounds.isEmpty() ? SGBounds{} : SGBounds{1, sg_rect(bounds)};
    value->visibilityKnown = true;
    return value->visible;
  } catch (...) {
    return {-1, {}};
  }
}
}
