#include "drag_data.h"
#include <cstring>
#include <new>
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

HRESULT SWDragData::QueryInterface(REFIID iid, void **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_IDataObject)) {
    *result = static_cast<IDataObject *>(this);
    AddRef();
    return S_OK;
  }
  return E_NOINTERFACE;
}
ULONG SWDragData::AddRef() { return ++references; }
ULONG SWDragData::Release() {
  ULONG left = --references;
  if (!left)
    delete this;
  return left;
}
HRESULT SWDragData::QueryGetData(FORMATETC *format) {
  if (!format)
    return E_POINTER;
  if (format->dwAspect != DVASPECT_CONTENT)
    return DV_E_DVASPECT;
  if (format->lindex != -1)
    return DV_E_LINDEX;
  if (!(format->tymed & TYMED_HGLOBAL))
    return DV_E_TYMED;
  for (const auto &value : formats)
    if (value.format == format->cfFormat)
      return S_OK;
  return DV_E_FORMATETC;
}
HRESULT SWDragData::GetData(FORMATETC *format, STGMEDIUM *medium) {
  if (!medium)
    return E_POINTER;
  *medium = {};
  HRESULT status = QueryGetData(format);
  if (FAILED(status))
    return status;
  for (const auto &value : formats) {
    if (value.format != format->cfFormat)
      continue;
    HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE, value.bytes.size());
    if (!memory)
      return E_OUTOFMEMORY;
    void *bytes = GlobalLock(memory);
    if (!bytes) {
      GlobalFree(memory);
      return E_OUTOFMEMORY;
    }
    memcpy(bytes, value.bytes.data(), value.bytes.size());
    GlobalUnlock(memory);
    medium->tymed = TYMED_HGLOBAL;
    medium->hGlobal = memory;
    return S_OK;
  }
  return DV_E_FORMATETC;
}
HRESULT SWDragData::GetDataHere(FORMATETC *, STGMEDIUM *) {
  return DATA_E_FORMATETC;
}
HRESULT SWDragData::GetCanonicalFormatEtc(FORMATETC *, FORMATETC *output) {
  if (!output)
    return E_POINTER;
  output->ptd = nullptr;
  return E_NOTIMPL;
}
HRESULT SWDragData::SetData(FORMATETC *, STGMEDIUM *, BOOL) {
  return E_NOTIMPL;
}
HRESULT SWDragData::EnumFormatEtc(DWORD direction, IEnumFORMATETC **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (direction != DATADIR_GET)
    return E_NOTIMPL;
  try {
    std::vector<FORMATETC> values;
    for (const auto &entry : formats)
      values.push_back(
          {entry.format, nullptr, DVASPECT_CONTENT, -1, TYMED_HGLOBAL});
    return SHCreateStdEnumFmtEtc(static_cast<UINT>(values.size()),
                                 values.data(), result);
  } catch (const std::bad_alloc &) {
    return E_OUTOFMEMORY;
  }
}
HRESULT SWDragData::DAdvise(FORMATETC *, DWORD, IAdviseSink *, DWORD *) {
  return OLE_E_ADVISENOTSUPPORTED;
}
HRESULT SWDragData::DUnadvise(DWORD) { return OLE_E_ADVISENOTSUPPORTED; }
HRESULT SWDragData::EnumDAdvise(IEnumSTATDATA **result) {
  if (result)
    *result = nullptr;
  return OLE_E_ADVISENOTSUPPORTED;
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
  if (size < sizeof(WCHAR) || size % sizeof(WCHAR))
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
      for (size_t n = length + 1; n < count; ++n)
        if (bytes[n])
          valid = false;
      if (valid)
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
