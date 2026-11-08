#ifndef SCOOP_NOTIFICATION_WINDOWS_H
#define SCOOP_NOTIFICATION_WINDOWS_H
#include "../notification/common.h"
#include "desktop.h"
#include <atomic>
#include <deque>
#include <memory>
#include <mutex>
#include <notificationactivationcallback.h>
#include <optional>
#include <roapi.h>
#include <unordered_map>
#include <windows.data.xml.dom.h>
#include <windows.ui.notifications.h>
#include <winstring.h>

namespace sn_toast = ABI::Windows::UI::Notifications;
namespace sn_xml = ABI::Windows::Data::Xml::Dom;

struct SNWinFailure {
  HRESULT code;
  int status;
};
inline void sn_check(HRESULT code, int status = SN_NATIVE_FAILURE) {
  if (FAILED(code))
    throw SNWinFailure{code, status};
}
std::wstring sn_wide(const std::string &text);
std::string sn_narrow(const std::wstring &text);
std::wstring sn_notification_xml(const SNContent *content);
bool sn_activation(const std::wstring &value, std::string &id,
                   std::string &action, std::string &payload);
bool sn_configured(const std::wstring &id, CLSID &clsid);

struct SNHString {
  HSTRING value = nullptr;
  explicit SNHString(const std::wstring &text) {
    sn_check(WindowsCreateString(text.data(), static_cast<UINT32>(text.size()),
                                 &value));
  }
  SNHString(const SNHString &) = delete;
  ~SNHString() { WindowsDeleteString(value); }
};
struct SNEventDelete {
  void operator()(SNEvent *event) const {
    scoop_notification_event_destroy(event);
  }
};
using SNEventPtr = std::unique_ptr<SNEvent, SNEventDelete>;
struct SNWinQueued {
  SNEventPtr event;
  std::string id;
  uint64_t generation;
};
struct SNWinQueue {
  std::mutex mutex;
  std::deque<SNWinQueued> events;
  HWND window = nullptr;
  bool closed = false, posted = false;
  HRESULT push(SNEvent *event, const std::string &id = {},
               uint64_t generation = 0);
  HRESULT wake();
  std::optional<SNWinQueued> pop();
};
struct SNToast {
  SDCom<sn_toast::IToastNotification> value;
  EventRegistrationToken failed{}, dismissed{};
  bool has_failed = false, has_dismissed = false;
  uint64_t generation = 0;
  ~SNToast() {
    if (has_failed)
      value->remove_Failed(failed);
    if (has_dismissed)
      value->remove_Dismissed(dismissed);
  }
};
struct SNRo {
  bool active = false;
  ~SNRo() {
    if (active)
      RoUninitialize();
  }
};
struct SNWindows {
  SNRo ro;
  std::wstring id;
  std::shared_ptr<SNWinQueue> queue = std::make_shared<SNWinQueue>();
  SDCom<IClassFactory> factory;
  SDCom<sn_toast::IToastNotifier> notifier;
  SDCom<sn_toast::IToastNotificationHistory> history;
  std::unordered_map<std::string, std::unique_ptr<SNToast>> notifications;
  HWND window = nullptr;
  DWORD registration = 0;
  bool configured = false;
  uint64_t generation = 1;
  SDCallback callback = nullptr;
  void *context = nullptr;
};

IClassFactory *sn_activation_factory(std::shared_ptr<SNWinQueue> queue,
                                     const std::wstring &id);
void sn_toast_handlers(SNWindows *owner, SNToast *toast, const std::string &id);
void sn_notifier(SNWindows *owner);
int sn_setting(SNWindows *owner);
constexpr UINT SN_WAKE = WM_APP + 93;
#endif
