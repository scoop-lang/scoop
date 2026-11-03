#include "graphics.h"

extern "C" {
SGTarget *scoop_graphics_offscreen_create(SGCanvas *source, int32_t width,
                                          int32_t height) {
  try {
    auto result = std::make_unique<SGTarget>();
    result->context = source->context;
    result->gl = source->gl;
    result->external = false;
    result->surface = source->surface->makeSurface(width, height);
    return result->surface ? result.release() : nullptr;
  } catch (...) {
    return nullptr;
  }
}
SGImage *scoop_graphics_target_composite_snapshot(SGTarget *target) {
  try {
    auto image = target->surface->makeImageSnapshot();
    return image ? new SGImage(std::move(image), target->context, target->gl)
                 : nullptr;
  } catch (...) {
    return nullptr;
  }
}
}
