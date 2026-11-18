#include "provider.h"
#include <algorithm>
#include <cmath>
#include <exception>

namespace swax::atspi {
const std::string &publicContent(const State &state) {
  if (!publicText(state))
    unsupported("Protected or non-text elements do not expose text");
  if (state.document) {
    const auto document = sw_text_document_state(state.document->pointer);
    if (!document.alive || document.revision != state.documentRevision ||
        document.length != static_cast<int64_t>(state.text[3]->size()))
      unavailable();
  }
  return *state.text[3];
}
int64_t textOffset(const State &state, int64_t index, bool allowEnd) {
  const auto &text = publicContent(state);
  auto result = utf8Offset(text, index, false);
  if (result < 0 || (!allowEnd && result == static_cast<int64_t>(text.size())))
    invalid("Text offset is out of range");
  return result;
}
GVariant *textAttributes(const State &state) {
  const bool editable =
      !(state.data.flags & SWAXReadOnly) && (state.data.actions & (4 | 32));
  return attributes({{"editable", editable ? "true" : "false"}});
}
struct Boxes {
  std::vector<Rect> values;
  std::exception_ptr failure;
};
static void boxVisitor(void *context, double x, double y, double width,
                       double height) noexcept {
  auto &result = *static_cast<Boxes *>(context);
  if (result.failure)
    return;
  try {
    result.values.push_back({x, y, width, height});
  } catch (...) {
    result.failure = std::current_exception();
  }
}
std::vector<Rect> textBoxes(const State &state, int64_t first, int64_t last) {
  if (!state.textGeometry)
    unavailable();
  const auto &layout = state.textGeometry->data;
  Boxes result;
  const auto status =
      layout.boxes(layout.context, first, last, &result, boxVisitor);
  if (result.failure)
    std::rethrow_exception(result.failure);
  if (status < 0)
    unavailable();
  return result.values;
}
static Rect joined(const std::vector<Rect> &boxes) {
  if (boxes.empty())
    unavailable();
  auto result = boxes.front();
  for (const auto &box : boxes) {
    const auto right = std::max(result.x + result.width, box.x + box.width);
    const auto bottom = std::max(result.y + result.height, box.y + box.height);
    result.x = std::min(result.x, box.x);
    result.y = std::min(result.y, box.y);
    result.width = right - result.x;
    result.height = bottom - result.y;
  }
  return result;
}
static int coordinate(double value) {
  if (!std::isfinite(value) || value < INT_MIN || value > INT_MAX)
    unavailable();
  return static_cast<int>(std::round(value));
}
GVariant *textExtents(Request &request, int64_t start, int64_t end,
                      uint32_t coordinates) {
  const auto rect = convertRect(
      request, joined(textBoxes(*request.state, start, end)), coordinates);
  return g_variant_new("(iiii)", coordinate(rect.x), coordinate(rect.y),
                       coordinate(rect.width), coordinate(rect.height));
}
static bool intersects(Rect box, Rect bounds, uint32_t xClip, uint32_t yClip) {
  if (box.x >= bounds.x + bounds.width || box.y >= bounds.y + bounds.height ||
      box.x + box.width <= bounds.x || box.y + box.height <= bounds.y)
    return false;
  return (!(xClip & 1) || box.x >= bounds.x) &&
         (!(xClip & 2) || box.x + box.width <= bounds.x + bounds.width) &&
         (!(yClip & 1) || box.y >= bounds.y) &&
         (!(yClip & 2) || box.y + box.height <= bounds.y + bounds.height);
}
GVariant *boundedText(Request &request, GVariant *parameters) {
  int x, y, width, height;
  uint32_t coordinates, xClip, yClip;
  g_variant_get(parameters, "(iiiiuuu)", &x, &y, &width, &height, &coordinates,
                &xClip, &yClip);
  if (width < 0 || height < 0 || xClip > 3 || yClip > 3)
    invalid();
  auto point = localPoint(
      request, {static_cast<double>(x), static_cast<double>(y)}, coordinates);
  Rect bounds{point.x, point.y, static_cast<double>(width),
              static_cast<double>(height)};
  const auto &content = publicContent(*request.state);
  if (!request.state->textGeometry)
    unavailable();
  auto &layout = request.state->textGeometry->data;
  const auto visible = layout.visible_range(layout.context);
  if (visible.status < 0)
    unavailable();
  std::vector<std::pair<int64_t, int64_t>> ranges;
  if (visible.status == 1) {
    for (int64_t offset = visible.start; offset < visible.end;) {
      const auto character = layout.range(layout.context, offset, 1, 0);
      if (character.status != 1 || character.end <= offset)
        unavailable();
      auto boxes = textBoxes(*request.state, character.start, character.end);
      bool included = false;
      for (const auto &box : boxes)
        included |= intersects(box, bounds, xClip, yClip);
      if (included) {
        if (!ranges.empty() && ranges.back().second == character.start)
          ranges.back().second = character.end;
        else
          ranges.emplace_back(character.start, character.end);
      }
      offset = character.end;
    }
  }
  GVariantBuilder result;
  g_variant_builder_init(&result, G_VARIANT_TYPE("a(iisv)"));
  for (const auto &range : ranges) {
    auto value =
        content.substr(static_cast<size_t>(range.first),
                       static_cast<size_t>(range.second - range.first));
    g_variant_builder_add(&result, "(ii@sv)",
                          integer(textUnits(content, range.first, false)),
                          integer(textUnits(content, range.second, false)),
                          string(value), g_variant_new_int32(0));
  }
  return g_variant_new("(@a(iisv))", g_variant_builder_end(&result));
}
} // namespace swax::atspi
