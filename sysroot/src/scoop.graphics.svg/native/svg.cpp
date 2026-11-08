#include "include/core/SkCanvas.h"
#include "include/core/SkColor.h"
#include "include/core/SkStream.h"
#include "modules/svg/include/SkSVGDOM.h"
#include "modules/svg/include/SkSVGRenderContext.h"

#include <cmath>
#include <cstdint>
#include <utility>

struct SSRect {
  double x, y, width, height;
};
struct SSColor {
  double red, green, blue, alpha;
};
struct SSInfo {
  double width, height;
  SSRect view_box;
  int32_t flags;
};

static bool positive_size(const SkSize &size) {
  return size.width() > 0 && size.height() > 0 && std::isfinite(size.width()) &&
         std::isfinite(size.height());
}

static bool absolute_length(const SkSVGLength &length) {
  using Unit = SkSVGLength::Unit;
  return length.unit() != Unit::kPercentage && length.unit() != Unit::kEMS &&
         length.unit() != Unit::kEXS;
}

struct SSDocument {
  sk_sp<SkSVGDOM> dom;
  SSInfo info{};

  explicit SSDocument(sk_sp<SkSVGDOM> value) : dom(std::move(value)) {
    const auto *root = dom->getRoot();
    if (absolute_length(root->getWidth()) &&
        absolute_length(root->getHeight())) {
      const auto size =
          root->intrinsicSize(SkSVGLengthContext(SkSize::Make(0, 0)));
      if (positive_size(size)) {
        info.width = size.width();
        info.height = size.height();
        info.flags |= 1;
      }
    }
    const auto &box = root->getViewBox();
    if (box && box->isFinite() && !box->isEmpty()) {
      info.view_box = {box->x(), box->y(), box->width(), box->height()};
      info.flags |= 2;
    }
  }
};

// Restore root presentation and container state even when rendering throws.
struct SSRootState {
  SkSVGDOM &dom;
  SkSize size;
  SkSVGProperty<SkSVGColorType, true> color;
  SkSVGProperty<SkSVGLength, true> stroke;

  explicit SSRootState(SkSVGDOM &value)
      : dom(value), size(value.containerSize()),
        color(value.getRoot()->getColor()),
        stroke(value.getRoot()->getStrokeWidth()) {}
  ~SSRootState() {
    dom.getRoot()->setColor(color);
    dom.getRoot()->setStrokeWidth(stroke);
    dom.setContainerSize(size);
  }
};

extern "C" {
void *scoop_graphics_svg_decode(const void *bytes, int64_t length) {
  try {
    if (length <= 0)
      return nullptr;
    auto stream = SkMemoryStream::MakeCopy(bytes, static_cast<size_t>(length));
    if (!stream)
      return nullptr;
    auto dom = SkSVGDOM::Builder().make(*stream);
    return dom && dom->getRoot() ? new SSDocument(std::move(dom)) : nullptr;
  } catch (...) {
    return nullptr;
  }
}

SSInfo scoop_graphics_svg_info(const void *pointer) {
  return static_cast<const SSDocument *>(pointer)->info;
}

void scoop_graphics_svg_drop(void *pointer) {
  delete static_cast<SSDocument *>(pointer);
}

int32_t scoop_graphics_svg_draw(void *pointer, void *canvas_pointer,
                                SSRect viewport, SSColor color, bool has_color,
                                double stroke, bool has_stroke,
                                double opacity) {
  try {
    const auto &document = *static_cast<SSDocument *>(pointer);
    auto *canvas = static_cast<SkCanvas *>(canvas_pointer);
    const float width = static_cast<float>(viewport.width);
    const float height = static_cast<float>(viewport.height);
    if (width == 0 || height == 0 || opacity == 0)
      return 1;
    const auto size =
        document.info.flags & 1
            ? SkSize::Make(document.info.width, document.info.height)
            : SkSize::Make(width, height);
    const float scale_x = width / size.width();
    const float scale_y = height / size.height();
    if (!std::isfinite(scale_x) || !std::isfinite(scale_y))
      return -1;
    SkAutoCanvasRestore canvas_state(canvas, true);
    SSRootState root_state(*document.dom);
    document.dom->setContainerSize(size);
    auto *root = document.dom->getRoot();
    if (has_color) {
      const SkColor4f rgba{
          static_cast<float>(color.red), static_cast<float>(color.green),
          static_cast<float>(color.blue), static_cast<float>(color.alpha)};
      root->setColor(SkSVGProperty<SkSVGColorType, true>(rgba.toSkColor()));
    }
    if (has_stroke)
      root->setStrokeWidth(
          SkSVGProperty<SkSVGLength, true>(SkSVGLength(stroke)));
    if (opacity != 1)
      canvas->saveLayerAlphaf(nullptr, static_cast<float>(opacity));
    canvas->translate(viewport.x, viewport.y);
    canvas->scale(scale_x, scale_y);
    document.dom->render(canvas);
    return 1;
  } catch (...) {
    return 0;
  }
}
}
