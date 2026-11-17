#include "provider.h"
#include <cmath>

namespace swax::atspi {
static Point origin(Request &request, uint32_t coordinates) {
  if (coordinates == 0) {
    auto rect = request.host->toScreen({});
    if (!rect)
      unsupported(
          "Global screen coordinates are unavailable on this window backend");
    return {rect->x, rect->y};
  }
  if (coordinates == 1) {
    auto *window = request.host->window;
    if (!window)
      unavailable();
    graphene_point_t zero = GRAPHENE_POINT_INIT(0, 0), offset{};
    if (!gtk_widget_compute_point(window->view, window->window, &zero, &offset))
      unavailable();
    return {offset.x, offset.y};
  }
  if (coordinates == 2) {
    auto parent = request.element->parent();
    auto state = parent ? parent->state() : nullptr;
    if (!state || !parent->reference.node)
      return {};
    if (!state->data.geometry)
      unavailable();
    return {-state->data.x, -state->data.y};
  }
  invalid("Unknown coordinate type");
}
Rect convertRect(Request &request, Rect value, uint32_t coordinates) {
  const auto offset = origin(request, coordinates);
  value.x += offset.x;
  value.y += offset.y;
  return value;
}
Point localPoint(Request &request, Point value, uint32_t coordinates) {
  const auto offset = origin(request, coordinates);
  return {value.x - offset.x, value.y - offset.y};
}
Rect extents(Request &request, uint32_t coordinates) {
  const auto &data = request.state->data;
  if (request.element == request.host->root)
    return convertRect(request, request.host->bounds(), coordinates);
  if (!data.geometry)
    unavailable();
  return convertRect(request, {data.x, data.y, data.width, data.height},
                     coordinates);
}
static int coordinate(double value) {
  if (!std::isfinite(value) || value < INT_MIN || value > INT_MAX)
    unavailable();
  return static_cast<int>(std::round(value));
}
GVariant *component(Request &request, const char *method,
                    GVariant *parameters) {
  if (!strcmp(method, "Contains") || !strcmp(method, "GetAccessibleAtPoint")) {
    int x, y;
    uint32_t coordinates;
    g_variant_get(parameters, "(iiu)", &x, &y, &coordinates);
    auto point = localPoint(
        request, {static_cast<double>(x), static_cast<double>(y)}, coordinates);
    const auto &state = *request.state;
    auto root = request.element == request.host->root;
    auto bounds =
        root ? request.host->bounds()
             : Rect{state.data.visible_x, state.data.visible_y,
                    state.data.visible_width, state.data.visible_height};
    bool inside = request.element->exposed() && point.x >= bounds.x &&
                  point.y >= bounds.y && point.x < bounds.x + bounds.width &&
                  point.y < bounds.y + bounds.height &&
                  (root || (state.hit &&
                            state.hit->data.contains(state.hit->data.context,
                                                     point.x, point.y) == 1));
    if (!strcmp(method, "Contains"))
      return g_variant_new("(b)", inside);
    auto tree = request.element->tree.lock();
    auto target = inside
                      ? tree->hit(point, request.element->reference.kind
                                             ? state.data.number
                                             : request.element->reference.node)
                      : nullptr;
    return g_variant_new("(@(so))",
                         request.host->reference(target, request.sender));
  }
  if (!strcmp(method, "GetExtents") || !strcmp(method, "GetPosition")) {
    uint32_t coordinates;
    g_variant_get(parameters, "(u)", &coordinates);
    auto rect = extents(request, coordinates);
    return !strcmp(method, "GetPosition")
               ? g_variant_new("(ii)", coordinate(rect.x), coordinate(rect.y))
               : g_variant_new("((iiii))", coordinate(rect.x),
                               coordinate(rect.y), coordinate(rect.width),
                               coordinate(rect.height));
  }
  if (!strcmp(method, "GetSize")) {
    auto rect = extents(request, 2);
    return g_variant_new("(ii)", coordinate(rect.width),
                         coordinate(rect.height));
  }
  if (!strcmp(method, "GetLayer"))
    return g_variant_new("(u)", request.state->data.role == 21 ||
                                        request.state->data.role == 17
                                    ? 5u
                                    : 3u);
  if (!strcmp(method, "GetMDIZOrder"))
    return g_variant_new("(n)", 0);
  if (!strcmp(method, "GrabFocus"))
    return g_variant_new("(b)", actionResult(request.element->perform(3)));
  if (!strcmp(method, "ScrollTo")) {
    uint32_t placement;
    g_variant_get(parameters, "(u)", &placement);
    if (placement > 6)
      invalid("Unknown scroll placement");
    if (placement != 6)
      unsupported("This element supports nearest reveal; text ranges also "
                  "support edge alignment");
    return g_variant_new("(b)", actionResult(request.element->perform(11)));
  }
  // Arbitrary native geometry and opacity are not writable semantic properties.
  unsupported();
}
} // namespace swax::atspi
