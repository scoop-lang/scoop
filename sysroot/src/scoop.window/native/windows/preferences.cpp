#include "../preferences.h"
#include "preferences_abi.h"
#include "window.h"
#include <new>
#include <objidl.h>
#include <roapi.h>
#include <winstring.h>

namespace {
class PreferencesHandler final : public SWPreferencesChanged {
  volatile LONG references = 1;
  SRWLOCK lock = SRWLOCK_INIT;
  HWND window;

public:
  explicit PreferencesHandler(HWND window) : window(window) {}

  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override {
    if (!result)
      return E_POINTER;
    *result = nullptr;
    if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_IAgileObject) ||
        IsEqualIID(iid, sw_settings_changed) ||
        IsEqualIID(iid, sw_scrollbars_changed)) {
      *result = static_cast<SWPreferencesChanged *>(this);
      AddRef();
      return S_OK;
    }
    return E_NOINTERFACE;
  }
  ULONG STDMETHODCALLTYPE AddRef() override {
    return InterlockedIncrement(&references);
  }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG remaining = InterlockedDecrement(&references);
    if (!remaining)
      delete this;
    return remaining;
  }
  HRESULT STDMETHODCALLTYPE Invoke(IInspectable *, IInspectable *) override {
    AcquireSRWLockShared(&lock);
    DWORD error = 0;
    if (window && !PostMessageW(window, SW_PREFERENCES, 0, 0))
      error = GetLastError();
    ReleaseSRWLockShared(&lock);
    return error ? HRESULT_FROM_WIN32(error) : S_OK;
  }
  void close() {
    // Invalidate the HWND before destruction, including in-flight callbacks.
    AcquireSRWLockExclusive(&lock);
    window = nullptr;
    ReleaseSRWLockExclusive(&lock);
  }
};

struct PreferencesState {
  bool initialized = false;
  PreferencesHandler *handler = nullptr;
  SWUISettings2 *scale = nullptr;
  SWUISettings4 *effects = nullptr;
  SWUISettings5 *scrollbars = nullptr;
  SWPreferencesToken scale_token{}, effects_token{}, scrollbars_token{};
};
} // namespace

int64_t sw_preferences_start(SWApplication *app) {
  auto *state = new (std::nothrow) PreferencesState;
  if (!state)
    return ERROR_NOT_ENOUGH_MEMORY;
  app->preferences = state;
  state->initialized = SUCCEEDED(RoInitialize(RO_INIT_SINGLETHREADED));
  if (!state->initialized)
    return 0;
  state->handler = new (std::nothrow) PreferencesHandler(app->hwnd);
  if (!state->handler)
    return ERROR_NOT_ENOUGH_MEMORY;
  static const WCHAR name[] = L"Windows.UI.ViewManagement.UISettings";
  HSTRING runtime_class = nullptr;
  IInspectable *settings = nullptr;
  HRESULT status =
      WindowsCreateString(name, ARRAYSIZE(name) - 1, &runtime_class);
  if (SUCCEEDED(status))
    status = RoActivateInstance(runtime_class, &settings);
  WindowsDeleteString(runtime_class);
  if (FAILED(status))
    return status == E_OUTOFMEMORY ? ERROR_NOT_ENOUGH_MEMORY : 0;

  if (SUCCEEDED(settings->QueryInterface(
          sw_ui_settings2, reinterpret_cast<void **>(&state->scale)))) {
    if (FAILED(state->scale->add_TextScaleFactorChanged(state->handler,
                                                        &state->scale_token))) {
      state->scale->Release();
      state->scale = nullptr;
    }
  }
  if (SUCCEEDED(settings->QueryInterface(
          sw_ui_settings4, reinterpret_cast<void **>(&state->effects)))) {
    if (FAILED(state->effects->add_AdvancedEffectsEnabledChanged(
            state->handler, &state->effects_token))) {
      state->effects->Release();
      state->effects = nullptr;
    }
  }
  if (SUCCEEDED(settings->QueryInterface(
          sw_ui_settings5, reinterpret_cast<void **>(&state->scrollbars)))) {
    if (FAILED(state->scrollbars->add_AutoHideScrollBarsChanged(
            state->handler, &state->scrollbars_token))) {
      state->scrollbars->Release();
      state->scrollbars = nullptr;
    }
  }
  settings->Release();
  return 0;
}

void sw_preferences_stop(SWApplication *app) {
  auto *state = static_cast<PreferencesState *>(app->preferences);
  if (!state)
    return;
  app->preferences = nullptr;
  if (state->handler)
    state->handler->close();
  if (state->scale) {
    state->scale->remove_TextScaleFactorChanged(state->scale_token);
    state->scale->Release();
  }
  if (state->effects) {
    state->effects->remove_AdvancedEffectsEnabledChanged(state->effects_token);
    state->effects->Release();
  }
  if (state->scrollbars) {
    state->scrollbars->remove_AutoHideScrollBarsChanged(
        state->scrollbars_token);
    state->scrollbars->Release();
  }
  if (state->handler)
    state->handler->Release();
  if (state->initialized)
    RoUninitialize();
  delete state;
}

extern "C" SWInteractionPreferences
scoop_window_interaction_preferences(void *pointer) {
  auto *app = static_cast<SWApplication *>(pointer);
  auto *state = static_cast<PreferencesState *>(app->preferences);
  SWInteractionPreferences value = sw_unknown_preferences();
  BOOL animations;
  if (SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &animations, 0))
    value.reduce_motion = !animations;
  HIGHCONTRASTW contrast{};
  contrast.cbSize = sizeof(contrast);
  if (SystemParametersInfoW(SPI_GETHIGHCONTRAST, sizeof(contrast), &contrast,
                            0))
    value.high_contrast = (contrast.dwFlags & HCF_HIGHCONTRASTON) != 0;
  UINT blink = GetCaretBlinkTime();
  if (blink)
    value.caret_blink_seconds = blink == INFINITE ? 0 : blink / 1000.0;
  UINT width;
  if (SystemParametersInfoW(SPI_GETCARETWIDTH, 0, &width, 0) &&
      width <= INT_MAX)
    value.caret_width_pixels = (int32_t)width;
  value.double_click_seconds = GetDoubleClickTime() / 1000.0;
  if (state) {
    DOUBLE scale;
    boolean enabled;
    if (state->scale && SUCCEEDED(state->scale->get_TextScaleFactor(&scale)))
      value.text_scale_factor = scale;
    if (state->effects &&
        SUCCEEDED(state->effects->get_AdvancedEffectsEnabled(&enabled)))
      value.reduce_transparency = !enabled;
    if (state->scrollbars &&
        SUCCEEDED(state->scrollbars->get_AutoHideScrollBars(&enabled)))
      value.auto_hide_scrollbars = enabled;
  }
  return value;
}
