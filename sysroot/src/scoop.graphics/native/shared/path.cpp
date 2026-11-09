#include "graphics.h"
#include "include/pathops/SkPathOps.h"

namespace {
void checkRing(SGBuilder *value) {
  if (value->ringChecked || value->roundRectCount != 2 ||
      value->path.fillType() != SkPathFillType::kEvenOdd)
    return;
  value->ringChecked = true;
  auto outer = value->roundRects[0], inner = value->roundRects[1];
  if (!outer.getBounds().contains(inner.getBounds()))
    std::swap(outer, inner);
  if (outer.isEmpty() || inner.isEmpty() ||
      !outer.getBounds().contains(inner.getBounds()))
    return;
  // Coincident edges need joint path coverage, not two independently
  // antialiased rounded rectangles.
  const auto &a = outer.getBounds();
  const auto &b = inner.getBounds();
  if (a.left() == b.left() || a.top() == b.top() || a.right() == b.right() ||
      a.bottom() == b.bottom())
    return;
  // Bounds containment alone is insufficient at rounded corners. PathOps is
  // performed once per unchanged builder, never during draw or hit testing.
  auto difference =
      Op(SkPathBuilder().addRRect(inner).detach(),
         SkPathBuilder().addRRect(outer).detach(), kDifference_SkPathOp);
  if (difference && difference->isEmpty())
    value->ring = std::make_shared<SGRRectRing>(SGRRectRing{outer, inner});
}
} // namespace

extern "C" {
void scoop_graphics_drop(SGObject *object) { delete object; }

SGBuilder *scoop_graphics_path_builder(int32_t rule) {
  try {
    auto value = std::make_unique<SGBuilder>();
    value->path.setFillType(rule ? SkPathFillType::kEvenOdd
                                 : SkPathFillType::kWinding);
    return value.release();
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_path_command(SGBuilder *value, int32_t kind, SGPoint a,
                                    SGPoint b, SGPoint c) {
  try {
    switch (kind) {
    case 0:
      value->path.moveTo(sg_point(a));
      break;
    case 1:
      value->path.lineTo(sg_point(a));
      break;
    case 2:
      value->path.quadTo(sg_point(a), sg_point(b));
      break;
    case 3:
      value->path.cubicTo(sg_point(a), sg_point(b), sg_point(c));
      break;
    case 4:
      value->path.close();
      break;
    case 5: {
      auto rule = value->path.fillType();
      value->path.reset().setFillType(rule);
      break;
    }
    default:
      return -1;
    }
    value->invalidateRoundRects(kind == 5);
    return 1;
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_path_rect(SGBuilder *value, SGRect rect, int32_t oval) {
  try {
    if (oval) {
      value->path.addOval(sg_rect(rect));
      value->addRoundRect(SkRRect::MakeOval(sg_rect(rect)));
    } else {
      value->path.addRect(sg_rect(rect));
      value->addRoundRect(SkRRect::MakeRect(sg_rect(rect)));
    }
    return 1;
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_path_arc(SGBuilder *value, SGRect rect, double start,
                                double sweep, bool move) {
  try {
    const double degrees = 180.0 / 3.14159265358979323846;
    float a = (float)(start * degrees), b = (float)(sweep * degrees);
    if (!std::isfinite(a) || !std::isfinite(b))
      return -1;
    value->path.arcTo(sg_rect(rect), a, b, move);
    value->invalidateRoundRects();
    return 1;
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_path_round_rect(SGBuilder *value, SGRect rect,
                                       SGPoint topLeft, SGPoint topRight,
                                       SGPoint bottomRight,
                                       SGPoint bottomLeft) {
  try {
    auto outline =
        sg_round_rect(rect, topLeft, topRight, bottomRight, bottomLeft);
    value->path.addRRect(outline);
    value->addRoundRect(outline);
    return 1;
  } catch (...) {
    return 0;
  }
}
SGPath *scoop_graphics_path_build(SGBuilder *value) {
  try {
    checkRing(value);
    return new SGPath(value->path.snapshot(), value->ring);
  } catch (...) {
    return nullptr;
  }
}
SGRect scoop_graphics_path_bounds(SGPath *value) {
  return sg_rect(value->path.computeTightBounds());
}
int32_t scoop_graphics_path_contains(SGPath *value, SGPoint point) {
  return value->path.contains(sg_point(point));
}
SGMeasure *scoop_graphics_path_measure(SGPath *value) {
  try {
    auto result = std::make_unique<SGMeasure>();
    SkContourMeasureIter iterator(value->path, false);
    while (auto contour = iterator.next()) {
      result->length += contour->length();
      result->contours.push_back(std::move(contour));
    }
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
double scoop_graphics_measure_length(SGMeasure *value) { return value->length; }
SGSample scoop_graphics_measure_sample(SGMeasure *value, double distance) {
  for (const auto &contour : value->contours) {
    if (distance <= contour->length()) {
      SkPoint position, tangent;
      if (contour->getPosTan((float)distance, &position, &tangent))
        return {1, sg_point(position), sg_point(tangent)};
      return {};
    }
    distance -= contour->length();
  }
  return {};
}
}
