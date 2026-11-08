#include "notification.h"

extern "C" int64_t scoop_window_windows_prepare_dpi(void);

HRESULT SNWinQueue::wake() {
  if (closed || posted || !window || events.empty())
    return S_OK;
  if (!PostMessageW(window, SN_WAKE, 0, 0))
    return HRESULT_FROM_WIN32(GetLastError());
  posted = true;
  return S_OK;
}
HRESULT SNWinQueue::push(SNEvent *pointer, const std::string &id,
                         uint64_t generation) {
  SNEventPtr event(pointer);
  std::lock_guard lock(mutex);
  if (closed)
    return S_OK;
  events.push_back({std::move(event), id, generation});
  return wake();
}
std::optional<SNWinQueued> SNWinQueue::pop() {
  std::lock_guard lock(mutex);
  if (events.empty())
    return std::nullopt;
  auto value = std::move(events.front());
  events.pop_front();
  return value;
}
static LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM wp,
                                    LPARAM lp) {
  SNWindows *owner =
      reinterpret_cast<SNWindows *>(GetWindowLongPtrW(window, GWLP_USERDATA));
  if (message == WM_NCCREATE) {
    owner = static_cast<SNWindows *>(
        reinterpret_cast<CREATESTRUCTW *>(lp)->lpCreateParams);
    SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(owner));
  }
  if (message == SN_WAKE && owner) {
    {
      std::lock_guard lock(owner->queue->mutex);
      owner->queue->posted = false;
    }
    if (owner->callback)
      owner->callback(owner->context, 0);
    return 0;
  }
  if (message == WM_NCDESTROY)
    SetWindowLongPtrW(window, GWLP_USERDATA, 0);
  return DefWindowProcW(window, message, wp, lp);
}
static void close(SNWindows *owner) {
  {
    std::lock_guard lock(owner->queue->mutex);
    owner->queue->closed = true;
    owner->queue->window = nullptr;
    owner->queue->events.clear();
  }
  owner->callback = nullptr;
  if (owner->registration)
    CoRevokeClassObject(owner->registration);
  owner->registration = 0;
  owner->notifications.clear();
  if (owner->window)
    DestroyWindow(owner->window);
  owner->window = nullptr;
}
SDResult scoop_notification_create(const uint8_t *id, int64_t length) {
  std::unique_ptr<SNWindows> owner;
  try {
    owner = std::make_unique<SNWindows>();
    if (!sd_wide(id, length, owner->id))
      return {nullptr, SD_INVALID};
    int64_t dpi_error = scoop_window_windows_prepare_dpi();
    if (dpi_error)
      return {nullptr, dpi_error};
    sn_check(RoInitialize(RO_INIT_SINGLETHREADED));
    owner->ro.active = true;
    WNDCLASSW type{};
    type.lpfnWndProc = window_proc;
    type.hInstance = GetModuleHandleW(nullptr);
    type.lpszClassName = L"Scoop.Notification.Owner";
    if (!RegisterClassW(&type) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS)
      sn_check(HRESULT_FROM_WIN32(GetLastError()));
    owner->window =
        CreateWindowExW(0, type.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE,
                        nullptr, type.hInstance, owner.get());
    if (!owner->window)
      sn_check(HRESULT_FROM_WIN32(GetLastError()));
    owner->queue->window = owner->window;
    CLSID clsid{};
    owner->configured = sn_configured(owner->id, clsid);
    if (owner->configured) {
      owner->factory.value = sn_activation_factory(owner->queue, owner->id);
      sn_check(CoRegisterClassObject(clsid, owner->factory.value,
                                     CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE,
                                     &owner->registration));
    }
    return {owner.release(), 0};
  } catch (const SNWinFailure &error) {
    if (owner)
      close(owner.get());
    return {nullptr, static_cast<uint32_t>(error.code)};
  } catch (const std::bad_alloc &) {
    if (owner)
      close(owner.get());
    return {nullptr, static_cast<uint32_t>(E_OUTOFMEMORY)};
  }
}
int64_t scoop_notification_attach(void *pointer, SDCallback callback,
                                  void *context) {
  SNWindows *owner = static_cast<SNWindows *>(pointer);
  owner->callback = callback;
  owner->context = context;
  std::lock_guard lock(owner->queue->mutex);
  return static_cast<uint32_t>(owner->queue->wake());
}
void scoop_notification_destroy(void *pointer) {
  std::unique_ptr<SNWindows> owner(static_cast<SNWindows *>(pointer));
  close(owner.get());
}
SNEvent *scoop_notification_next_event(void *pointer) {
  SNWindows *owner = static_cast<SNWindows *>(pointer);
  auto item = owner->queue->pop();
  if (!item)
    return nullptr;
  if (item->generation) {
    auto found = owner->notifications.find(item->id);
    if (found != owner->notifications.end() &&
        found->second->generation == item->generation)
      owner->notifications.erase(found);
  }
  return item->event.release();
}
void sn_notifier(SNWindows *owner) {
  if (owner->notifier.value)
    return;
  SDCom<sn_toast::IToastNotificationManagerStatics> manager;
  SNHString name(L"Windows.UI.Notifications.ToastNotificationManager");
  sn_check(RoGetActivationFactory(
      name.value, __uuidof(sn_toast::IToastNotificationManagerStatics),
      reinterpret_cast<void **>(manager.put())));
  SNHString id(owner->id);
  sn_check(manager->CreateToastNotifierWithId(id.value, owner->notifier.put()));
}
int sn_setting(SNWindows *owner) {
  if (!owner->configured)
    return SN_UNCONFIGURED;
  sn_notifier(owner);
  sn_toast::NotificationSetting setting;
  HRESULT result = owner->notifier->get_Setting(&setting);
  // A registered desktop app has no notification settings record until its
  // first Show.
  if (result == HRESULT_FROM_WIN32(ERROR_NOT_FOUND))
    return SN_AVAILABLE;
  sn_check(result);
  return setting == sn_toast::NotificationSetting_Enabled ? SN_AVAILABLE
                                                          : SN_SETTING_DISABLED;
}
int64_t scoop_notification_settings(void *pointer, int64_t request,
                                    int32_t permission) {
  SNWindows *owner = static_cast<SNWindows *>(pointer);
  SNEventPtr event(sn_event(permission ? SN_PERMISSION : SN_SETTINGS, request));
  try {
    event->data.detail = sn_setting(owner);
    if (owner->configured)
      event->data.capabilities = SN_ACTIONS | SN_REPLACE | SN_CAN_WITHDRAW |
                                 SN_ACTIVATION | SN_CLOSE_EVENTS |
                                 SN_COLD_ACTIVATION;
  } catch (const SNWinFailure &error) {
    event->data.detail = SN_SERVICE_UNAVAILABLE;
  } catch (const std::bad_alloc &) {
    return static_cast<uint32_t>(E_OUTOFMEMORY);
  }
  try {
    return static_cast<uint32_t>(owner->queue->push(event.release()));
  } catch (const std::bad_alloc &) {
    return static_cast<uint32_t>(E_OUTOFMEMORY);
  }
}
