#include "graphics.h"
#include <atomic>

extern "C" int32_t scoop_graphics_geometry_hit(const SGGeometry *, SGPoint);

namespace {
struct HitSnapshot {
  std::atomic<uint64_t> references{1};
  const SGGeometry geometry;

  explicit HitSnapshot(const SGGeometry &source) : geometry(source) {}
};

void retain(void *pointer) {
  auto *snapshot = static_cast<HitSnapshot *>(pointer);
  snapshot->references.fetch_add(1, std::memory_order_relaxed);
}

void release(void *pointer) {
  auto *snapshot = static_cast<HitSnapshot *>(pointer);
  if (snapshot->references.fetch_sub(1, std::memory_order_acq_rel) == 1)
    delete snapshot;
}

int32_t contains(void *pointer, double x, double y) {
  const auto *snapshot = static_cast<const HitSnapshot *>(pointer);
  return scoop_graphics_geometry_hit(&snapshot->geometry, {x, y});
}
} // namespace

struct SGNativeGeometryHitTest {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  int32_t (*contains)(void *, double, double);
};

extern "C" SGNativeGeometryHitTest
scoop_graphics_geometry_export_hit(const SGGeometry *geometry) {
  try {
    return {new HitSnapshot(*geometry), retain, release, contains};
  } catch (...) {
    return {nullptr, retain, release, contains};
  }
}
