#ifndef SCOOP_TRAY_WINDOWS_H
#define SCOOP_TRAY_WINDOWS_H

#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#define UNICODE
#include <windows.h>

#include "../tray.h"
#include <limits.h>
#include <math.h>
#include <shellapi.h>
#include <shellscalingapi.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <windowsx.h>

typedef struct STImage {
  struct STImage *next;
  int width, height;
  uint8_t pixels[];
} STImage;

typedef struct {
  STCallback callback;
  void *context;
  HWND hwnd;
  HMENU menu;
  HICON icon;
  STImage *images, *pending_images;
  WCHAR tooltip[128];
  UINT taskbar_created;
  bool started, available, closing, presenting;
  bool has_anchor;
  POINT anchor;
} STTray;

int64_t st_error(void);
void st_available(STTray *tray, bool available);
void st_free_images(STImage *images);
int64_t st_refresh_image(STTray *tray);
STGeometry scoop_tray_geometry(void *pointer);
void scoop_tray_cancel(void *pointer);

#endif
