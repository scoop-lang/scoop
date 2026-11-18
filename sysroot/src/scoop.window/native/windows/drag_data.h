#ifndef SCOOP_WINDOW_WINDOWS_DRAG_DATA_H
#define SCOOP_WINDOW_WINDOWS_DRAG_DATA_H

#include "../drag_drop.h"
#include "window.h"
#include <oleidl.h>
#include <string>
#include <vector>

struct SWDragFormat {
  CLIPFORMAT format;
  std::vector<uint8_t> bytes;
};

class SWDragData final : public IDataObject {
  ULONG references = 1;

public:
  std::vector<SWDragFormat> formats;
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID, void **) override;
  ULONG STDMETHODCALLTYPE AddRef() override;
  ULONG STDMETHODCALLTYPE Release() override;
  HRESULT STDMETHODCALLTYPE GetData(FORMATETC *, STGMEDIUM *) override;
  HRESULT STDMETHODCALLTYPE GetDataHere(FORMATETC *, STGMEDIUM *) override;
  HRESULT STDMETHODCALLTYPE QueryGetData(FORMATETC *) override;
  HRESULT STDMETHODCALLTYPE GetCanonicalFormatEtc(FORMATETC *,
                                                  FORMATETC *) override;
  HRESULT STDMETHODCALLTYPE SetData(FORMATETC *, STGMEDIUM *, BOOL) override;
  HRESULT STDMETHODCALLTYPE EnumFormatEtc(DWORD, IEnumFORMATETC **) override;
  HRESULT STDMETHODCALLTYPE DAdvise(FORMATETC *, DWORD, IAdviseSink *,
                                    DWORD *) override;
  HRESULT STDMETHODCALLTYPE DUnadvise(DWORD) override;
  HRESULT STDMETHODCALLTYPE EnumDAdvise(IEnumSTATDATA **) override;
};

struct SWDropPayload {
  std::vector<std::vector<uint8_t>> items;
};

CLIPFORMAT sw_drag_url_format();
bool sw_drag_absolute_url(const std::wstring &value);
int sw_drag_formats(IDataObject *data);
bool sw_drag_read(IDataObject *data, int format, SWDropPayload &payload);
int sw_drag_local(SWWindow *window, IDataObject *data);

#endif
