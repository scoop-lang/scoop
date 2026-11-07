#include "menu.h"

static int64_t image_bitmap(SWMenuEntry *entry, UINT dpi) {
  double scale = dpi / 96.0;
  double target_width = ceil(entry->image_width * scale);
  double target_height = ceil(entry->image_height * scale);
  if (target_width > INT_MAX || target_height > INT_MAX)
    return ERROR_ARITHMETIC_OVERFLOW;
  int width = (int)fmax(1, target_width), height = (int)fmax(1, target_height);
  if ((size_t)width > SIZE_MAX / 4 / (size_t)height)
    return ERROR_ARITHMETIC_OVERFLOW;
  SWMenuImage *chosen = NULL;
  double distance = HUGE_VAL;
  for (SWMenuImage *image = entry->images; image; image = image->next) {
    double value = fabs(log((double)image->width / width)) +
                   fabs(log((double)image->height / height));
    if (value < distance) {
      chosen = image;
      distance = value;
    }
  }
  if (!chosen)
    return 0;
  BITMAPINFO info = {.bmiHeader = {.biSize = sizeof(BITMAPINFOHEADER),
                                   .biWidth = width,
                                   .biHeight = -height,
                                   .biPlanes = 1,
                                   .biBitCount = 32,
                                   .biCompression = BI_RGB}};
  uint8_t *pixels = NULL;
  HBITMAP bitmap =
      CreateDIBSection(NULL, &info, DIB_RGB_COLORS, (void **)&pixels, NULL, 0);
  if (!bitmap)
    return sw_menu_error();
  for (int y = 0; y < height; ++y) {
    int source_y = (int)((int64_t)y * chosen->height / height);
    for (int x = 0; x < width; ++x) {
      int source_x = (int)((int64_t)x * chosen->width / width);
      const uint8_t *source =
          chosen->pixels + ((size_t)source_y * chosen->width + source_x) * 4;
      uint8_t *target = pixels + ((size_t)y * width + x) * 4;
      target[0] = (uint8_t)((source[2] * source[3] + 127) / 255);
      target[1] = (uint8_t)((source[1] * source[3] + 127) / 255);
      target[2] = (uint8_t)((source[0] * source[3] + 127) / 255);
      target[3] = source[3];
    }
  }
  MENUITEMINFOW item = {
      .cbSize = sizeof(item), .fMask = MIIM_BITMAP, .hbmpItem = bitmap};
  if (!SetMenuItemInfoW(entry->parent, entry->position, TRUE, &item)) {
    int64_t error = sw_menu_error();
    DeleteObject(bitmap);
    return error;
  }
  if (entry->bitmap)
    DeleteObject(entry->bitmap);
  entry->bitmap = bitmap;
  return 0;
}

int64_t sw_menu_images(SWMenu *menu, UINT dpi) {
  for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next) {
    int64_t error = image_bitmap(entry, dpi);
    if (error)
      return error;
  }
  return 0;
}

void sw_menu_dpi(SWWindow *window, UINT dpi) {
  if (window->menu_bar) {
    int64_t error = sw_menu_images(window->menu_bar, dpi);
    if (error)
      sw_fail(window->application, (DWORD)error);
    else if (!DrawMenuBar(window->hwnd))
      sw_fail(window->application, GetLastError());
  }
}
