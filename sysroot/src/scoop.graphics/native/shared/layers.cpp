#include "graphics.h"
#include "include/core/SkBBHFactory.h"

bool SGCanvas::saveOpacityLayer(const SkPaint &paint) {
  if (!context || recording || canvas->isClipEmpty())
    return false;
  auto layer = std::make_unique<SGOpacityLayer>();
  layer->target = canvas;
  layer->depth = clips.size();
  layer->paint = paint;
  // Record in device space, retaining the current CTM. The parent's precise
  // clip remains in place at replay; its bounds conservatively limit recording.
  SkRect bounds = SkRect::Make(canvas->getDeviceClipBounds());
  SkRTreeFactory factory;
  SkCanvas *commands = layer->recorder.beginRecording(bounds, &factory);
  commands->clipRect(bounds);
  commands->setMatrix(canvas->getLocalToDevice());
  opacityLayers.push_back(std::move(layer));
  canvas->save();
  canvas = commands;
  return true;
}

void SGCanvas::restore() {
  if (!opacityLayers.empty() && opacityLayers.back()->depth == clips.size()) {
    auto layer = std::move(opacityLayers.back());
    opacityLayers.pop_back();
    auto picture = layer->recorder.finishRecordingAsPicture();
    canvas = layer->target;
    if (picture && !picture->cullRect().isEmpty()) {
      // Skia bounds include strokes and filters. Reserve one physical pixel
      // for antialias coverage without rounding in logical coordinates.
      SkRect bounds = picture->cullRect().makeOutset(1, 1);
      canvas->resetMatrix();
      canvas->saveLayer(&bounds, &layer->paint);
      picture->playback(canvas);
      canvas->restore();
    }
  }
  canvas->restore();
  clips.pop_back();
}

SGCanvas::~SGCanvas() {
  while (clips.size() > 1)
    restore();
  canvas->restoreToCount(base);
}
