#pragma once

#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include "include/core/SkContourMeasure.h"
#include "include/core/SkData.h"
#include "include/core/SkImage.h"
#include "include/core/SkImageFilter.h"
#include "include/core/SkPathBuilder.h"
#include "include/core/SkPicture.h"
#include "include/core/SkPictureRecorder.h"
#include "include/core/SkSurface.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include <cmath>
#include <cstdint>
#include <memory>
#include <vector>

struct SGPoint {
  double x, y;
};
struct SGRect {
  double x, y, width, height;
};
struct SGColor {
  double red, green, blue, alpha;
};
struct SGTransform {
  double a, b, c, d, tx, ty;
};
struct SGSample {
  int32_t found;
  SGPoint position, tangent;
};
struct SGBounds {
  int32_t found;
  SGRect rect;
};

struct SGObject {
  virtual ~SGObject() = default;
};
struct SGPath : SGObject {
  SkPath path;
  explicit SGPath(SkPath p) : path(std::move(p)) {}
};
struct SGBuilder : SGObject {
  SkPathBuilder path;
};
struct SGMeasure : SGObject {
  std::vector<sk_sp<SkContourMeasure>> contours;
  double length = 0;
};
struct SGPaint : SGObject {
  SkPaint paint;
  std::vector<SkColor4f> colors;
  std::vector<float> positions;
};
struct SGImage : SGObject {
  sk_sp<GrDirectContext> context;
  sk_sp<SkImage> image;
  explicit SGImage(sk_sp<SkImage> i, sk_sp<GrDirectContext> c = nullptr)
      : context(std::move(c)), image(std::move(i)) {}
};
struct SGDrawing : SGObject {
  sk_sp<GrDirectContext> context;
  sk_sp<SkPicture> picture;
  SGDrawing(sk_sp<SkPicture> p, sk_sp<GrDirectContext> c)
      : context(std::move(c)), picture(std::move(p)) {}
};
struct SGFilter : SGObject {
  sk_sp<SkImageFilter> filter;
  explicit SGFilter(sk_sp<SkImageFilter> f) : filter(std::move(f)) {}
};
struct SGContext : SGObject {
  sk_sp<GrDirectContext> context;
};
struct SGTarget : SGObject {
  sk_sp<SkSurface> surface;
  sk_sp<GrDirectContext> context;
  bool external = true;
};
struct SGCanvas : SGObject {
  sk_sp<SkSurface> surface;
  sk_sp<GrDirectContext> context;
  std::shared_ptr<SkPictureRecorder> recording;
  SkCanvas *canvas;
  float scale;
  SkRect viewport;
  int base;
  std::vector<std::vector<SkPath>> clips;
  SGCanvas(sk_sp<SkSurface> s, sk_sp<GrDirectContext> c, float dpi)
      : surface(std::move(s)), context(std::move(c)),
        canvas(surface->getCanvas()), scale(dpi),
        viewport(
            SkRect::MakeWH(surface->width() / dpi, surface->height() / dpi)),
        base(canvas->getSaveCount()), clips(1) {
    canvas->save();
    canvas->scale(dpi, dpi);
  }
  explicit SGCanvas(const SGCanvas &parent)
      : surface(parent.surface), context(parent.context),
        recording(parent.recording), canvas(parent.canvas), scale(parent.scale),
        viewport(parent.viewport), base(canvas->getSaveCount()),
        clips(1, parent.clips.back()) {
    canvas->save();
  }
  SGCanvas(sk_sp<SkSurface> s, sk_sp<GrDirectContext> c,
           std::shared_ptr<SkPictureRecorder> r, SkRect bounds)
      : surface(std::move(s)), context(std::move(c)), recording(std::move(r)),
        canvas(recording->getRecordingCanvas()), scale(1), viewport(bounds),
        base(canvas->getSaveCount()), clips(1) {
    canvas->save();
    canvas->clipRect(bounds);
  }
  ~SGCanvas() override { canvas->restoreToCount(base); }
  SkMatrix logical() const {
    return SkMatrix::Concat(SkMatrix::Scale(1 / scale, 1 / scale),
                            canvas->getLocalToDeviceAs3x3());
  }
};

inline bool sg_compatible(const SGCanvas *canvas,
                          const sk_sp<GrDirectContext> &context) {
  return !context || canvas->context == context;
}
struct SGGeometry : SGObject {
  SkMatrix matrix, inverse;
  bool invertible;
  std::vector<SkPath> clips;
  SkPath shape;
  SkRect viewport;
  bool visibilityKnown = false;
  SGBounds visible = {};
};

inline SkPoint sg_point(SGPoint p) { return {(float)p.x, (float)p.y}; }
inline SGPoint sg_point(SkPoint p) { return {p.x(), p.y()}; }
inline SkRect sg_rect(SGRect r) {
  return SkRect::MakeLTRB((float)r.x, (float)r.y, (float)(r.x + r.width),
                          (float)(r.y + r.height));
}
inline SGRect sg_rect(SkRect r) {
  return {r.x(), r.y(), r.width(), r.height()};
}
inline SkColor4f sg_color(SGColor c) {
  return {(float)c.red, (float)c.green, (float)c.blue, (float)c.alpha};
}
inline SkMatrix sg_matrix(SGTransform m) {
  return SkMatrix::MakeAll((float)m.a, (float)m.c, (float)m.tx, (float)m.b,
                           (float)m.d, (float)m.ty, 0, 0, 1);
}
inline SkImageInfo sg_info(int width, int height,
                           SkAlphaType alpha = kPremul_SkAlphaType) {
  return SkImageInfo::Make(width, height, kRGBA_8888_SkColorType, alpha,
                           SkColorSpace::MakeSRGB());
}
