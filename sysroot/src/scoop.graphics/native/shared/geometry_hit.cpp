#include "graphics.h"
#include "include/pathops/SkPathOps.h"
#include <atomic>

extern "C" int32_t scoop_graphics_geometry_hit(const SGGeometry *, SGPoint);
extern "C" SGSample scoop_graphics_geometry_point(const SGGeometry *, SGPoint,
                                                  bool);
extern "C" SGRect scoop_graphics_geometry_rect_map(const SGGeometry *, SGRect);
extern "C" SGBounds scoop_graphics_geometry_visible(SGGeometry *);

struct SGNativeGeometryPoint {
  int32_t status;
  SGPoint point;
};

namespace {
struct GeometrySnapshot {
  std::atomic<uint64_t> references{1};
  const SGGeometry geometry;

  explicit GeometrySnapshot(const SGGeometry &source) : geometry(source) {}
};

void retain(void *pointer) {
  auto *snapshot = static_cast<GeometrySnapshot *>(pointer);
  snapshot->references.fetch_add(1, std::memory_order_relaxed);
}

void release(void *pointer) {
  auto *snapshot = static_cast<GeometrySnapshot *>(pointer);
  if (snapshot->references.fetch_sub(1, std::memory_order_acq_rel) == 1)
    delete snapshot;
}

int32_t contains(void *pointer, double x, double y) {
  const auto *snapshot = static_cast<const GeometrySnapshot *>(pointer);
  return scoop_graphics_geometry_hit(&snapshot->geometry, {x, y});
}

bool scalar_finite(double value) {
  return std::isfinite(value) && std::isfinite((float)value);
}

SGNativeGeometryPoint to_local(void *pointer, double x, double y) {
  if (!scalar_finite(x) || !scalar_finite(y))
    return {-1, {}};
  const auto *snapshot = static_cast<const GeometrySnapshot *>(pointer);
  const auto result =
      scoop_graphics_geometry_point(&snapshot->geometry, {x, y}, true);
  return {result.found, result.position};
}

SGBounds map_rect(void *pointer, SGRect rect, int32_t clipped) {
  if ((clipped != 0 && clipped != 1) || !scalar_finite(rect.x) ||
      !scalar_finite(rect.y) || !scalar_finite(rect.width) ||
      !scalar_finite(rect.height) || !scalar_finite(rect.x + rect.width) ||
      !scalar_finite(rect.y + rect.height) || rect.width < 0 || rect.height < 0)
    return {-1, {}};
  try {
    const auto *snapshot = static_cast<const GeometrySnapshot *>(pointer);
    if (!clipped) {
      const auto result =
          scoop_graphics_geometry_rect_map(&snapshot->geometry, rect);
      if (!scalar_finite(result.x) || !scalar_finite(result.y) ||
          !scalar_finite(result.width) || !scalar_finite(result.height))
        return {0, {}};
      return {1, result};
    }
    SGGeometry query(snapshot->geometry);
    if (!query.invertible)
      return {0, {}};
    const auto shape =
        Op(query.shape, SkPathBuilder().addRect(sg_rect(rect)).detach(),
           kIntersect_SkPathOp);
    if (!shape)
      return {-2, {}};
    query.shape = *shape;
    query.visibilityKnown = false;
    auto result = scoop_graphics_geometry_visible(&query);
    if (result.found < 0)
      result.found = -2;
    return result;
  } catch (...) {
    return {-2, {}};
  }
}
} // namespace

struct SGNativeGeometry {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  int32_t (*contains)(void *, double, double);
  SGNativeGeometryPoint (*to_local)(void *, double, double);
  SGBounds (*map_rect)(void *, SGRect, int32_t);
};

extern "C" SGNativeGeometry
scoop_graphics_geometry_export(const SGGeometry *geometry) {
  try {
    return {new GeometrySnapshot(*geometry),
            retain,
            release,
            contains,
            to_local,
            map_rect};
  } catch (...) {
    return {nullptr, retain, release, contains, to_local, map_rect};
  }
}
