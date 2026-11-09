#include "graphics.h"
#include "include/core/SkBBHFactory.h"

namespace {
struct Recording final : SGObject {
  sk_sp<SkSurface> surface;
  sk_sp<GrDirectContext> context;
  SGDeviceInfo device;
  std::shared_ptr<SGRecording> recording = std::make_shared<SGRecording>();
  SkRect bounds;
};
} // namespace

extern "C" {
SGObject *scoop_graphics_recording_create(SGCanvas *source, SGRect bounds) {
  try {
    auto result = std::make_unique<Recording>();
    result->surface = source->surface;
    result->context = source->context;
    result->device = source->device;
    result->gl = source->gl;
    result->bounds = sg_rect(bounds);
    SkRTreeFactory factory;
    result->recording->recorder.beginRecording(result->bounds, &factory);
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
SGCanvas *scoop_graphics_recording_canvas(Recording *value) {
  try {
    auto result =
        std::make_unique<SGCanvas>(value->surface, value->context,
                                   value->recording, value->bounds, value->gl);
    result->device = value->device;
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
SGDrawing *scoop_graphics_recording_finish(Recording *value) {
  try {
    auto picture = value->recording->recorder.finishRecordingAsPicture();
    return picture
               ? new SGDrawing(std::move(picture), value->context,
                               value->recording->independentTiles, value->gl)
               : nullptr;
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_canvas_drawing(SGCanvas *value, SGDrawing *drawing) {
  try {
    if (!sg_compatible(value, drawing->context))
      return -1;
    if (value->recording && !drawing->independentTiles)
      value->recording->independentTiles = false;
    value->canvas->drawPicture(drawing->picture);
    return 1;
  } catch (...) {
    return 0;
  }
}
bool scoop_graphics_drawing_compatible(SGCanvas *value, SGDrawing *drawing) {
  return sg_compatible(value, drawing->context);
}
bool scoop_graphics_drawing_independent_tiles(SGDrawing *drawing) {
  return drawing->independentTiles;
}
}
