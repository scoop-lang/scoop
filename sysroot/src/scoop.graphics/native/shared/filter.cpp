#include "graphics.h"
#include "include/effects/SkImageFilters.h"

extern "C" {
SGFilter *scoop_graphics_filter_blur(double x, double y) {
  try {
    // Skia can represent the valid zero-sigma identity by a null filter.
    return new SGFilter(
        SkImageFilters::Blur((float)x, (float)y, SkTileMode::kDecal, nullptr));
  } catch (...) {
    return nullptr;
  }
}
SGFilter *scoop_graphics_filter_shadow(SGPoint offset, double x, double y,
                                       SGColor color, bool only) {
  try {
    auto space = SkColorSpace::MakeSRGB();
    auto filter =
        only ? SkImageFilters::DropShadowOnly((float)offset.x, (float)offset.y,
                                              (float)x, (float)y,
                                              sg_color(color), space, nullptr)
             : SkImageFilters::DropShadow((float)offset.x, (float)offset.y,
                                          (float)x, (float)y, sg_color(color),
                                          space, nullptr);
    return new SGFilter(std::move(filter));
  } catch (...) {
    return nullptr;
  }
}
SGFilter *scoop_graphics_filter_then(SGFilter *first, SGFilter *next) {
  try {
    const auto *owner = first->context ? first : next;
    return new SGFilter(SkImageFilters::Compose(next->filter, first->filter),
                        owner->context, owner->gl);
  } catch (...) {
    return nullptr;
  }
}
bool scoop_graphics_filters_compatible(SGFilter *first, SGFilter *next) {
  return !first->context || !next->context || first->context == next->context;
}
int32_t scoop_graphics_canvas_backdrop(SGCanvas *target, SGPath *outline,
                                       SGFilter *filter) {
  if (!sg_compatible(target, filter->context))
    return -1;
  try {
    if (target->recording)
      target->recording->independentTiles = false;
    if (filter->filter) {
      SkAutoCanvasRestore state(target->canvas, true);
      target->canvas->clipPath(outline->path, true);
      SkPaint restore;
      restore.setBlendMode(SkBlendMode::kSrc);
      target->canvas->saveLayer(
          SkCanvas::SaveLayerRec(nullptr, &restore, filter->filter.get(), 0));
    }
    return 1;
  } catch (...) {
    return 0;
  }
}
}
