#ifndef SCOOP_WINDOW_PREFERENCES_ABI_H
#define SCOOP_WINDOW_PREFERENCES_ABI_H

#include <inspectable.h>

// Public UISettings ABI, also usable with the LLVM-MinGW WinRT headers.
// Method order and interface IDs are from windows.ui.viewmanagement.h.
struct SWPreferencesToken {
  INT64 value;
};
struct SWPreferencesChanged : IUnknown {
  virtual HRESULT STDMETHODCALLTYPE Invoke(IInspectable *, IInspectable *) = 0;
};
struct SWUISettings2 : IInspectable {
  virtual HRESULT STDMETHODCALLTYPE get_TextScaleFactor(DOUBLE *) = 0;
  virtual HRESULT STDMETHODCALLTYPE
  add_TextScaleFactorChanged(SWPreferencesChanged *, SWPreferencesToken *) = 0;
  virtual HRESULT
      STDMETHODCALLTYPE remove_TextScaleFactorChanged(SWPreferencesToken) = 0;
};
struct SWUISettings4 : IInspectable {
  virtual HRESULT STDMETHODCALLTYPE get_AdvancedEffectsEnabled(boolean *) = 0;
  virtual HRESULT STDMETHODCALLTYPE add_AdvancedEffectsEnabledChanged(
      SWPreferencesChanged *, SWPreferencesToken *) = 0;
  virtual HRESULT STDMETHODCALLTYPE
      remove_AdvancedEffectsEnabledChanged(SWPreferencesToken) = 0;
};
struct SWUISettings5 : IInspectable {
  virtual HRESULT STDMETHODCALLTYPE get_AutoHideScrollBars(boolean *) = 0;
  virtual HRESULT STDMETHODCALLTYPE add_AutoHideScrollBarsChanged(
      SWPreferencesChanged *, SWPreferencesToken *) = 0;
  virtual HRESULT STDMETHODCALLTYPE
      remove_AutoHideScrollBarsChanged(SWPreferencesToken) = 0;
};

static const IID sw_ui_settings2 = {
    0xbad82401,
    0x2721,
    0x44f9,
    {0xbb, 0x91, 0x2b, 0xb2, 0x28, 0xbe, 0x44, 0x2f}};
static const IID sw_ui_settings4 = {
    0x52bb3002,
    0x919b,
    0x4d6b,
    {0x9b, 0x78, 0x8d, 0xd6, 0x6f, 0xf4, 0xb9, 0x3b}};
static const IID sw_ui_settings5 = {
    0x5349d588,
    0x0cb5,
    0x5f05,
    {0xbd, 0x34, 0x70, 0x6b, 0x32, 0x31, 0xf0, 0xbd}};
static const IID sw_settings_changed = {
    0x2dbdba9d,
    0x20da,
    0x519d,
    {0x90, 0x78, 0x09, 0xf8, 0x35, 0xbc, 0x5b, 0xc7}};
static const IID sw_scrollbars_changed = {
    0x808aef30,
    0x2660,
    0x51b0,
    {0x9c, 0x11, 0xf7, 0x5d, 0xd4, 0x20, 0x06, 0xb4}};

#endif
