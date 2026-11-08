#include "desktop.h"
#include <cmath>
#include <cstdlib>
#include <limits>

HICON sd_icon(SDImage *image, int width, int height) {
  if (width <= 0 || height <= 0) {
    SetLastError(ERROR_INVALID_PARAMETER);
    return nullptr;
  }
  SDPixels *chosen = nullptr;
  double distance = std::numeric_limits<double>::infinity();
  for (SDPixels *pixels = image->pixels; pixels; pixels = pixels->next) {
    double score = std::abs((double)pixels->width - width) +
                   std::abs((double)pixels->height - height);
    if (score < distance) {
      distance = score;
      chosen = pixels;
    }
  }
  if (!chosen) {
    SetLastError(ERROR_INVALID_DATA);
    return nullptr;
  }
  BITMAPINFO info{};
  info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  info.bmiHeader.biWidth = width;
  info.bmiHeader.biHeight = -height;
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  info.bmiHeader.biCompression = BI_RGB;
  uint8_t *pixels = nullptr;
  HBITMAP color =
      CreateDIBSection(nullptr, &info, DIB_RGB_COLORS,
                       reinterpret_cast<void **>(&pixels), nullptr, 0);
  if (!color)
    return nullptr;
  for (int y = 0; y < height; ++y) {
    size_t source_y = (size_t)y * chosen->height / height;
    for (int x = 0; x < width; ++x) {
      size_t source_x = (size_t)x * chosen->width / width;
      const uint8_t *source =
          chosen->bytes + (source_y * chosen->width + source_x) * 4;
      uint8_t *destination = pixels + ((size_t)y * width + x) * 4;
      destination[0] = (uint8_t)((source[2] * source[3] + 127) / 255);
      destination[1] = (uint8_t)((source[1] * source[3] + 127) / 255);
      destination[2] = (uint8_t)((source[0] * source[3] + 127) / 255);
      destination[3] = source[3];
    }
  }
  size_t stride = ((size_t)width + 15) / 16 * 2;
  void *maskBits = calloc(height, stride);
  HBITMAP mask =
      maskBits ? CreateBitmap(width, height, 1, 1, maskBits) : nullptr;
  free(maskBits);
  ICONINFO data{};
  data.fIcon = TRUE;
  data.hbmMask = mask;
  data.hbmColor = color;
  HICON icon = mask ? CreateIconIndirect(&data) : nullptr;
  DWORD error = icon ? 0 : GetLastError();
  if (mask)
    DeleteObject(mask);
  DeleteObject(color);
  if (!icon)
    SetLastError(error ? error : ERROR_NOT_ENOUGH_MEMORY);
  return icon;
}
