#include "graphics.h"
#include "include/core/SkBBHFactory.h"

namespace {
struct Recording final : SGObject {
  sk_sp<SkSurface> surface;
  sk_sp<GrDirectContext> context;
  std::shared_ptr<SkPictureRecorder> recorder =
      std::make_shared<SkPictureRecorder>();
  SkRect bounds;
};
} // namespace

extern "C" {
SGObject *scoop_graphics_recording_create(SGCanvas *source, SGRect bounds) {
  try {
    auto result = std::make_unique<Recording>();
    result->surface = source->surface;
    result->context = source->context;
    result->bounds = sg_rect(bounds);
    SkRTreeFactory factory;
    result->recorder->beginRecording(result->bounds, &factory);
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
SGCanvas *scoop_graphics_recording_canvas(Recording *value) {
  try {
    return new SGCanvas(value->surface, value->context, value->recorder,
                        value->bounds);
  } catch (...) {
    return nullptr;
  }
}
SGDrawing *scoop_graphics_recording_finish(Recording *value) {
  try {
    auto picture = value->recorder->finishRecordingAsPicture();
    return picture ? new SGDrawing(std::move(picture), value->context)
                   : nullptr;
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_canvas_drawing(SGCanvas *value, SGDrawing *drawing) {
  try {
    if (!sg_compatible(value, drawing->context))
      return -1;
    value->canvas->drawPicture(drawing->picture);
    return 1;
  } catch (...) {
    return 0;
  }
}
bool scoop_graphics_drawing_compatible(SGCanvas *value, SGDrawing *drawing) {
  return sg_compatible(value, drawing->context);
}
}
