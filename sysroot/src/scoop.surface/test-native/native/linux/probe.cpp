#include "include/core/SkCanvas.h"
#include "include/core/SkColorSpace.h"
#include <cstdint>

extern "C" uint64_t scoop_surface_test_canvas_pixel(SkCanvas *canvas, int32_t x,
                                                    int32_t y) {
  uint8_t bytes[4] = {};
  auto info =
      SkImageInfo::Make(1, 1, kRGBA_8888_SkColorType, kUnpremul_SkAlphaType,
                        SkColorSpace::MakeSRGB());
  if (!canvas->readPixels(info, bytes, 4, x, y))
    return UINT64_MAX;
  return (uint64_t(bytes[0]) << 24) | (uint64_t(bytes[1]) << 16) |
         (uint64_t(bytes[2]) << 8) | bytes[3];
}
