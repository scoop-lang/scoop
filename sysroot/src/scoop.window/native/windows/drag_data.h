#ifndef SCOOP_WINDOW_WINDOWS_DRAG_DATA_H
#define SCOOP_WINDOW_WINDOWS_DRAG_DATA_H

#include "../drag_drop.h"
#include "window.h"
#include <oleidl.h>
#include <string>
#include <vector>

HRESULT sw_drag_set_bytes(IDataObject *data, CLIPFORMAT format,
                          const uint8_t *bytes, size_t length);

int64_t sw_drag_set_preview(IDataObject *data, UINT dpi, int32_t width,
                            int32_t height, const uint8_t *rgba,
                            double logical_width, double logical_height,
                            double hot_x, double hot_y);

struct SWDropPayload {
  std::vector<std::vector<uint8_t>> items;
};

CLIPFORMAT sw_drag_url_format();
bool sw_drag_absolute_url(const std::wstring &value);
int sw_drag_formats(IDataObject *data);
bool sw_drag_read(IDataObject *data, int format, SWDropPayload &payload);
int sw_drag_local(SWWindow *window, IDataObject *data);

#endif
