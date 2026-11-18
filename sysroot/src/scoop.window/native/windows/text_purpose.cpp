#include "text_internal.h"
#include <inputscope.h>

namespace {
class TextPurpose final : public ITfInputScope {
  ULONG references = 1;
  InputScope value;

public:
  explicit TextPurpose(InputScope value) : value(value) {}
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override {
    if (!result)
      return E_POINTER;
    *result = nullptr;
    if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfInputScope)) {
      *result = static_cast<ITfInputScope *>(this);
      AddRef();
      return S_OK;
    }
    return E_NOINTERFACE;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++references; }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG left = --references;
    if (!left)
      delete this;
    return left;
  }
  HRESULT STDMETHODCALLTYPE GetInputScopes(InputScope **scopes,
                                           UINT *count) override {
    if (!scopes || !count)
      return E_POINTER;
    *count = 0;
    *scopes = static_cast<InputScope *>(CoTaskMemAlloc(sizeof(InputScope)));
    if (!*scopes)
      return E_OUTOFMEMORY;
    **scopes = value;
    *count = 1;
    return S_OK;
  }
  HRESULT STDMETHODCALLTYPE GetPhrase(BSTR **phrases, UINT *count) override {
    if (!phrases || !count)
      return E_POINTER;
    *phrases = nullptr;
    *count = 0;
    return E_NOTIMPL;
  }
  HRESULT STDMETHODCALLTYPE GetRegularExpression(BSTR *value) override {
    return absent(value);
  }
  HRESULT STDMETHODCALLTYPE GetSRGS(BSTR *value) override {
    return absent(value);
  }
  HRESULT STDMETHODCALLTYPE GetXML(BSTR *value) override {
    return absent(value);
  }
  static HRESULT absent(BSTR *value) {
    if (!value)
      return E_POINTER;
    *value = nullptr;
    return E_NOTIMPL;
  }
};
} // namespace

HRESULT sw_text_input_scope(int purpose, IUnknown **result) {
  static const InputScope values[] = {
      IS_DEFAULT,  IS_DIGITS,
      IS_NUMBER,   IS_EMAIL_SMTPEMAILADDRESS,
      IS_URL,      IS_TELEPHONE_FULLTELEPHONENUMBER,
      IS_PASSWORD, IS_SEARCH};
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (purpose < 0 || static_cast<size_t>(purpose) >= ARRAYSIZE(values))
    return E_INVALIDARG;
  auto *value = new (std::nothrow) TextPurpose(values[purpose]);
  if (!value)
    return E_OUTOFMEMORY;
  *result = static_cast<ITfInputScope *>(value);
  return S_OK;
}

extern "C" int64_t scoop_window_text_purpose(SWWindow *window,
                                             int64_t connection,
                                             int32_t purpose) {
  auto *state = static_cast<SWTextState *>(window->text);
  if (!window->ime_allowed || !state || state->closed ||
      state->connection != connection)
    return -4;
  if (purpose < 0 || purpose > 7)
    return ERROR_INVALID_PARAMETER;
  int64_t error = sw_keyboard_start(window);
  if (error)
    return error;
  if (state->purpose != purpose) {
    state->purpose = purpose;
    sw_text_store_notify_purpose(state);
  }
  return 0;
}
