#ifndef SCOOP_WINDOW_WINDOWS_KEYBOARD_ABI_H
#define SCOOP_WINDOW_WINDOWS_KEYBOARD_ABI_H

#include "window.h"
#include <inspectable.h>

// Public Windows SDK ABI, kept local like preferences_abi.h so the backend
// does not depend on generated C++/WinRT projection headers.
struct SWInputPaneInterop : IInspectable {
  virtual HRESULT STDMETHODCALLTYPE GetForWindow(HWND, REFIID, void **) = 0;
};
struct SWInputPane2 : IInspectable {
  virtual HRESULT STDMETHODCALLTYPE TryShow(boolean *) = 0;
  virtual HRESULT STDMETHODCALLTYPE TryHide(boolean *) = 0;
};
static const GUID sw_input_pane_interop = {
    0x75cf2c57,
    0x9195,
    0x4931,
    {0x83, 0x32, 0xf0, 0xb4, 0x09, 0xe9, 0x16, 0xaf}};
static const GUID sw_input_pane2 = {
    0x8a6b3f26,
    0x7090,
    0x4793,
    {0x94, 0x4c, 0xc3, 0xf2, 0xcd, 0xe2, 0x62, 0x76}};

#endif
