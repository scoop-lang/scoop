#include "drag_data.h"
#include <climits>
#include <cmath>
#include <cstring>
#include <new>
#include <shellapi.h>
#include <shlobj.h>

CLIPFORMAT sw_drag_url_format() {
  return static_cast<CLIPFORMAT>(
      RegisterClipboardFormatW(L"UniformResourceLocatorW"));
}

bool sw_drag_absolute_url(const std::wstring &value) {
  size_t colon = value.find(L':');
  if (colon == std::wstring::npos || colon == 0)
    return false;
  for (size_t n = 0; n < colon; ++n) {
    wchar_t c = value[n];
    bool letter = (c >= L'A' && c <= L'Z') || (c >= L'a' && c <= L'z');
    if (!letter && (n == 0 || !((c >= L'0' && c <= L'9') || c == L'+' ||
                                c == L'-' || c == L'.')))
      return false;
  }
  return true;
}

HRESULT sw_drag_set_bytes(IDataObject *data, CLIPFORMAT format,
                          const uint8_t *bytes, size_t length) {
  STGMEDIUM medium = {};
  medium.tymed = TYMED_HGLOBAL;
  medium.hGlobal = GlobalAlloc(GMEM_MOVEABLE, length);
  if (!medium.hGlobal)
    return E_OUTOFMEMORY;
  void *mapped = GlobalLock(medium.hGlobal);
  if (!mapped) {
    GlobalFree(medium.hGlobal);
    return E_OUTOFMEMORY;
  }
  memcpy(mapped, bytes, length);
  GlobalUnlock(medium.hGlobal);
  FORMATETC type = {format, nullptr, DVASPECT_CONTENT, -1, TYMED_HGLOBAL};
  HRESULT status = data->SetData(&type, &medium, TRUE);
  if (FAILED(status))
    ReleaseStgMedium(&medium);
  return status;
}

int64_t sw_drag_set_preview(IDataObject *data, UINT dpi, int32_t width,
                            int32_t height, const uint8_t *rgba,
                            double logical_width, double logical_height,
                            double hot_x, double hot_y) {
  double scale = dpi / 96.0;
  double w = std::ceil(logical_width * scale),
         h = std::ceil(logical_height * scale);
  double x = std::round(hot_x * scale), y = std::round(hot_y * scale);
  if (w < 1 || h < 1 || w > INT_MAX || h > INT_MAX || x < INT_MIN ||
      x > INT_MAX || y < INT_MIN || y > INT_MAX)
    return -8;
  IDragSourceHelper *helper = nullptr;
  HRESULT status =
      CoCreateInstance(CLSID_DragDropHelper, nullptr, CLSCTX_INPROC_SERVER,
                       IID_PPV_ARGS(&helper));
  if (FAILED(status))
    return static_cast<uint32_t>(status);
  SHDRAGIMAGE image = {};
  image.sizeDragImage = {static_cast<LONG>(w), static_cast<LONG>(h)};
  image.ptOffset = {static_cast<LONG>(x), static_cast<LONG>(y)};
  image.crColorKey = 0xffffffffU;
  BITMAPINFO info = {};
  info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  info.bmiHeader.biWidth = image.sizeDragImage.cx;
  info.bmiHeader.biHeight = -image.sizeDragImage.cy;
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  void *bits = nullptr;
  image.hbmpDragImage =
      CreateDIBSection(nullptr, &info, DIB_RGB_COLORS, &bits, nullptr, 0);
  if (!image.hbmpDragImage) {
    helper->Release();
    return ERROR_NOT_ENOUGH_MEMORY;
  }
  // InitializeFromBitmap wants straight BGRA, including for partially
  // transparent glyphs.
  auto *dest = static_cast<uint8_t *>(bits);
  for (int row = 0; row < image.sizeDragImage.cy; ++row) {
    size_t sy = static_cast<int64_t>(row) * height / image.sizeDragImage.cy;
    for (int col = 0; col < image.sizeDragImage.cx; ++col) {
      size_t sx = static_cast<int64_t>(col) * width / image.sizeDragImage.cx;
      const uint8_t *p = rgba + (sy * width + sx) * 4;
      *dest++ = p[2];
      *dest++ = p[1];
      *dest++ = p[0];
      *dest++ = p[3];
    }
  }
  status = helper->InitializeFromBitmap(&image, data);
  if (FAILED(status))
    DeleteObject(image.hbmpDragImage);
  helper->Release();
  return static_cast<uint32_t>(status);
}

int sw_drag_formats(IDataObject *data) {
  data->AddRef();
  int result = 0;
  CLIPFORMAT formats[] = {CF_UNICODETEXT, sw_drag_url_format(), CF_HDROP};
  for (int n = 0; n < 3; ++n) {
    FORMATETC format = {formats[n], nullptr, DVASPECT_CONTENT, -1,
                        TYMED_HGLOBAL};
    if (formats[n] && data->QueryGetData(&format) == S_OK)
      result |= 1 << n;
  }
  data->Release();
  return result;
}

static bool read_wide(HGLOBAL memory, std::wstring &value) {
  SIZE_T size = GlobalSize(memory);
  if (size < sizeof(WCHAR))
    return false;
  const WCHAR *bytes = static_cast<const WCHAR *>(GlobalLock(memory));
  if (!bytes)
    return false;
  bool valid = false;
  try {
    size_t count = size / sizeof(WCHAR), length = 0;
    while (length < count && bytes[length])
      ++length;
    if (length < count) {
      valid = true;
      value.assign(bytes, length);
    }
  } catch (...) {
    GlobalUnlock(memory);
    throw;
  }
  GlobalUnlock(memory);
  return valid;
}

bool sw_drag_read(IDataObject *data, int code, SWDropPayload &payload) {
  CLIPFORMAT type = code == 1   ? CF_UNICODETEXT
                    : code == 2 ? sw_drag_url_format()
                                : CF_HDROP;
  FORMATETC format = {type, nullptr, DVASPECT_CONTENT, -1, TYMED_HGLOBAL};
  STGMEDIUM medium = {};
  if (FAILED(data->GetData(&format, &medium)))
    return false;
  bool valid = false;
  try {
    if (medium.tymed == TYMED_HGLOBAL && medium.hGlobal) {
      if (code == 1 || code == 2) {
        std::wstring text;
        if (read_wide(medium.hGlobal, text) && text.size() <= INT_MAX &&
            (code != 2 || sw_drag_absolute_url(text))) {
          int64_t length = 0;
          uint8_t *bytes =
              sw_utf8(text.data(), static_cast<int>(text.size()), &length);
          if (bytes) {
            try {
              payload.items.emplace_back(bytes, bytes + length);
            } catch (...) {
              free(bytes);
              throw;
            }
            free(bytes);
            valid = true;
          }
        }
      } else if (code == 3) {
        HDROP drop = reinterpret_cast<HDROP>(medium.hGlobal);
        UINT count = DragQueryFileW(drop, 0xffffffff, nullptr, 0);
        valid = count != 0;
        for (UINT n = 0; valid && n < count; ++n) {
          UINT length = DragQueryFileW(drop, n, nullptr, 0);
          if (!length || length == UINT_MAX) {
            valid = false;
            break;
          }
          std::vector<WCHAR> path(static_cast<size_t>(length) + 1);
          if (DragQueryFileW(drop, n, path.data(), length + 1) != length) {
            valid = false;
            break;
          }
          const auto *bytes = reinterpret_cast<const uint8_t *>(path.data());
          payload.items.emplace_back(
              bytes, bytes + static_cast<size_t>(length) * sizeof(WCHAR));
        }
      }
    }
  } catch (const std::bad_alloc &) {
    valid = false;
  }
  ReleaseStgMedium(&medium);
  return valid;
}

extern "C" int64_t scoop_window_drop_item_count(SWDropPayload *payload) {
  return static_cast<int64_t>(payload->items.size());
}
extern "C" SWDropBytes scoop_window_drop_text(SWDropPayload *payload,
                                              int64_t index) {
  if (index < 0 || static_cast<size_t>(index) >= payload->items.size())
    return {nullptr, -1};
  const auto &value = payload->items[static_cast<size_t>(index)];
  return {value.data(), static_cast<int64_t>(value.size())};
}
extern "C" SWDropBytes scoop_window_drop_path(SWDropPayload *payload,
                                              int64_t index) {
  return scoop_window_drop_text(payload, index);
}
