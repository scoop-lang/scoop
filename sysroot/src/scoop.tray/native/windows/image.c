#include "tray.h"

void st_free_images(STImage *images) {
  while (images) {
    STImage *next = images->next;
    free(images);
    images = next;
  }
}

static HICON make_icon(STTray *tray, STImage *images) {
  STGeometry geometry = scoop_tray_geometry(tray);
  HMONITOR monitor;
  if (!geometry.error) {
    POINT point = {(LONG)geometry.x, (LONG)geometry.y};
    monitor = MonitorFromPoint(point, MONITOR_DEFAULTTOPRIMARY);
  } else {
    monitor = MonitorFromWindow(tray->hwnd, MONITOR_DEFAULTTOPRIMARY);
  }
  UINT dpi = 96, vertical = 96;
  GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &dpi, &vertical);
  int width = GetSystemMetricsForDpi(SM_CXSMICON, dpi);
  int height = GetSystemMetricsForDpi(SM_CYSMICON, dpi);
  if (width <= 0 || height <= 0) {
    SetLastError(ERROR_INVALID_DATA);
    return NULL;
  }
  STImage *chosen = NULL;
  double distance = INFINITY;
  for (STImage *image = images; image; image = image->next) {
    double value = fabs((double)image->width - width) +
                   fabs((double)image->height - height);
    if (value < distance) {
      chosen = image;
      distance = value;
    }
  }
  if (!chosen) {
    SetLastError(ERROR_INVALID_DATA);
    return NULL;
  }
  BITMAPINFO info = {.bmiHeader = {.biSize = sizeof(BITMAPINFOHEADER),
                                   .biWidth = width,
                                   .biHeight = -height,
                                   .biPlanes = 1,
                                   .biBitCount = 32,
                                   .biCompression = BI_RGB}};
  uint8_t *pixels = NULL;
  HBITMAP color =
      CreateDIBSection(NULL, &info, DIB_RGB_COLORS, (void **)&pixels, NULL, 0);
  if (!color)
    return NULL;
  for (int y = 0; y < height; ++y) {
    size_t source_y = (size_t)y * (size_t)chosen->height / (size_t)height;
    for (int x = 0; x < width; ++x) {
      size_t source_x = (size_t)x * (size_t)chosen->width / (size_t)width;
      const uint8_t *source =
          chosen->pixels + (source_y * (size_t)chosen->width + source_x) * 4;
      uint8_t *target = pixels + ((size_t)y * (size_t)width + (size_t)x) * 4;
      target[0] = (uint8_t)((source[2] * source[3] + 127) / 255);
      target[1] = (uint8_t)((source[1] * source[3] + 127) / 255);
      target[2] = (uint8_t)((source[0] * source[3] + 127) / 255);
      target[3] = source[3];
    }
  }
  size_t stride = ((size_t)width + 15) / 16 * 2;
  void *mask_bits = calloc((size_t)height, stride);
  HBITMAP mask =
      mask_bits ? CreateBitmap(width, height, 1, 1, mask_bits) : NULL;
  free(mask_bits);
  ICONINFO icon_info = {.fIcon = TRUE, .hbmMask = mask, .hbmColor = color};
  HICON icon = mask ? CreateIconIndirect(&icon_info) : NULL;
  DWORD error = icon ? 0 : GetLastError();
  if (mask)
    DeleteObject(mask);
  DeleteObject(color);
  if (!icon)
    SetLastError(error ? error : ERROR_NOT_ENOUGH_MEMORY);
  return icon;
}

static int64_t publish(STTray *tray, STImage *images) {
  HICON icon = make_icon(tray, images);
  if (!icon)
    return st_error();
  if (tray->available) {
    NOTIFYICONDATAW data = {.cbSize = sizeof(data),
                            .hWnd = tray->hwnd,
                            .uID = 1,
                            .uFlags = NIF_ICON,
                            .hIcon = icon};
    if (!Shell_NotifyIconW(NIM_MODIFY, &data)) {
      DestroyIcon(icon);
      st_available(tray, false);
      return -2;
    }
  }
  if (tray->icon)
    DestroyIcon(tray->icon);
  tray->icon = icon;
  return 0;
}

int64_t st_refresh_image(STTray *tray) { return publish(tray, tray->images); }

int64_t scoop_tray_image_begin(void *pointer, double width, double height,
                               int32_t template) {
  STTray *tray = pointer;
  st_free_images(tray->pending_images);
  tray->pending_images = NULL;
  return 0;
}

int64_t scoop_tray_image_add(void *pointer, int32_t width, int32_t height,
                             const uint8_t *pixels) {
  STTray *tray = pointer;
  if (width <= 0 || height <= 0 ||
      (size_t)width > (SIZE_MAX - sizeof(STImage)) / 4 / (size_t)height)
    return ERROR_ARITHMETIC_OVERFLOW;
  size_t bytes = (size_t)width * (size_t)height * 4;
  STImage *image = malloc(sizeof(*image) + bytes);
  if (!image)
    return ERROR_NOT_ENOUGH_MEMORY;
  image->width = width;
  image->height = height;
  memcpy(image->pixels, pixels, bytes);
  image->next = tray->pending_images;
  tray->pending_images = image;
  return 0;
}

int64_t scoop_tray_image_commit(void *pointer) {
  STTray *tray = pointer;
  int64_t error = publish(tray, tray->pending_images);
  if (error)
    return error;
  st_free_images(tray->images);
  tray->images = tray->pending_images;
  tray->pending_images = NULL;
  return 0;
}
