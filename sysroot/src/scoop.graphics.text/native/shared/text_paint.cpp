#include "include/core/SkBBHFactory.h"
#include "include/core/SkPicture.h"
#include "include/core/SkPictureRecorder.h"
#include "text.h"

extern "C" STCaret scoop_graphics_text_paint_bounds(STTextLayout *layout) {
  try {
    if (!layout->paint_bounds_known) {
      SkPictureRecorder recorder;
      SkRTreeFactory factory;
      constexpr float limit = std::numeric_limits<float>::max();
      auto canvas = recorder.beginRecording(
          SkRect::MakeLTRB(-limit, -limit, limit, limit), &factory);
      layout->paragraph->paint(canvas, 0, 0);
      auto picture = recorder.finishRecordingAsPicture();
      if (!picture)
        return {};
      const auto bounds = picture->cullRect();
      if (!bounds.isFinite() || !std::isfinite(bounds.width()) ||
          !std::isfinite(bounds.height()))
        return {};
      layout->paint_bounds = bounds;
      layout->paint_bounds_known = true;
    }
    return {st_rect(layout->paint_bounds), 1};
  } catch (...) {
    return {};
  }
}
