#include "notification.h"

template <class Interface> class SNUnknown : public Interface {
  std::atomic<ULONG> references{1};

protected:
  virtual ~SNUnknown() = default;

public:
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **value) override {
    if (!value)
      return E_POINTER;
    *value = nullptr;
    if (iid != IID_IUnknown && iid != __uuidof(Interface) &&
        iid != __uuidof(IAgileObject))
      return E_NOINTERFACE;
    *value = static_cast<Interface *>(this);
    AddRef();
    return S_OK;
  }
  ULONG STDMETHODCALLTYPE AddRef() override { return ++references; }
  ULONG STDMETHODCALLTYPE Release() override {
    ULONG count = --references;
    if (!count)
      delete this;
    return count;
  }
};

class SNActivation final : public SNUnknown<INotificationActivationCallback> {
  std::shared_ptr<SNWinQueue> queue;
  std::wstring id;

public:
  SNActivation(std::shared_ptr<SNWinQueue> queue, std::wstring id)
      : queue(std::move(queue)), id(std::move(id)) {}
  HRESULT STDMETHODCALLTYPE Activate(LPCWSTR application, LPCWSTR arguments,
                                     const NOTIFICATION_USER_INPUT_DATA *data,
                                     ULONG count) override {
    if (!application || !arguments || id != application)
      return E_INVALIDARG;
    try {
      std::string identifier, action, payload;
      if (!sn_activation(arguments, identifier, action, payload))
        return E_INVALIDARG;
      SNEventPtr event(sn_event(SN_ACTIVATED, 0));
      sn_event_set(event.get(), 0, identifier.c_str());
      sn_event_set(event.get(), 1, action.c_str());
      sn_event_set(event.get(), 2, payload.c_str());
      return queue->push(event.release());
    } catch (const SNWinFailure &error) {
      return error.code;
    } catch (const std::bad_alloc &) {
      return E_OUTOFMEMORY;
    }
  }
};
class SNFactory final : public SNUnknown<IClassFactory> {
  std::shared_ptr<SNWinQueue> queue;
  std::wstring id;

public:
  SNFactory(std::shared_ptr<SNWinQueue> queue, std::wstring id)
      : queue(std::move(queue)), id(std::move(id)) {}
  HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown *outer, REFIID iid,
                                           void **value) override {
    if (outer)
      return CLASS_E_NOAGGREGATION;
    try {
      auto activation = new SNActivation(queue, id);
      HRESULT status = activation->QueryInterface(iid, value);
      activation->Release();
      return status;
    } catch (const std::bad_alloc &) {
      return E_OUTOFMEMORY;
    }
  }
  // Application owns process lifetime; COM callers do not terminate it.
  HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override { return S_OK; }
};
IClassFactory *sn_activation_factory(std::shared_ptr<SNWinQueue> queue,
                                     const std::wstring &id) {
  return new SNFactory(std::move(queue), id);
}

using SNFailedInterface =
    __FITypedEventHandler_2_Windows__CUI__CNotifications__CToastNotification_Windows__CUI__CNotifications__CToastFailedEventArgs;
using SNDismissedInterface =
    __FITypedEventHandler_2_Windows__CUI__CNotifications__CToastNotification_Windows__CUI__CNotifications__CToastDismissedEventArgs;

class SNFailed final : public SNUnknown<SNFailedInterface> {
  std::shared_ptr<SNWinQueue> queue;
  std::string id;
  uint64_t generation;

public:
  SNFailed(std::shared_ptr<SNWinQueue> queue, std::string id,
           uint64_t generation)
      : queue(std::move(queue)), id(std::move(id)), generation(generation) {}
  HRESULT STDMETHODCALLTYPE
  Invoke(sn_toast::IToastNotification *sender,
         sn_toast::IToastFailedEventArgs *args) override {
    try {
      HRESULT error = E_FAIL;
      args->get_ErrorCode(&error);
      SNEventPtr event(sn_event(SN_FAILED, 0));
      sn_event_set(event.get(), 0, id.c_str());
      sn_event_error(event.get(), SN_NATIVE_FAILURE,
                     static_cast<uint32_t>(error), nullptr);
      return queue->push(event.release(), id, generation);
    } catch (const std::bad_alloc &) {
      return E_OUTOFMEMORY;
    }
  }
};
class SNDismissed final : public SNUnknown<SNDismissedInterface> {
  std::shared_ptr<SNWinQueue> queue;
  std::string id;
  uint64_t generation;

public:
  SNDismissed(std::shared_ptr<SNWinQueue> queue, std::string id,
              uint64_t generation)
      : queue(std::move(queue)), id(std::move(id)), generation(generation) {}
  HRESULT STDMETHODCALLTYPE
  Invoke(sn_toast::IToastNotification *sender,
         sn_toast::IToastDismissedEventArgs *args) override {
    try {
      sn_toast::ToastDismissalReason reason;
      HRESULT status = args->get_Reason(&reason);
      SNEventPtr event(sn_event(SN_CLOSED, 0));
      if (SUCCEEDED(status)) {
        if (reason == sn_toast::ToastDismissalReason_TimedOut)
          event->data.detail = 1;
        else if (reason == sn_toast::ToastDismissalReason_UserCanceled)
          event->data.detail = 2;
        else if (reason == sn_toast::ToastDismissalReason_ApplicationHidden)
          event->data.detail = 3;
      }
      sn_event_set(event.get(), 0, id.c_str());
      return queue->push(event.release(), id, generation);
    } catch (const std::bad_alloc &) {
      return E_OUTOFMEMORY;
    }
  }
};
void sn_toast_handlers(SNWindows *owner, SNToast *toast,
                       const std::string &id) {
  SDCom<SNFailedInterface> failed;
  failed.value = new SNFailed(owner->queue, id, toast->generation);
  sn_check(toast->value->add_Failed(failed.value, &toast->failed));
  toast->has_failed = true;
  SDCom<SNDismissedInterface> dismissed;
  dismissed.value = new SNDismissed(owner->queue, id, toast->generation);
  sn_check(toast->value->add_Dismissed(dismissed.value, &toast->dismissed));
  toast->has_dismissed = true;
}
