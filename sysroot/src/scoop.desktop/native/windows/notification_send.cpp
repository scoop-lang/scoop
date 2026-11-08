#include "notification.h"

static void available(SNWindows *owner) {
  int state = sn_setting(owner);
  if (state == SN_UNCONFIGURED)
    throw SNWinFailure{S_OK, SN_NOT_CONFIGURED};
  if (state == SN_SETTING_DISABLED)
    throw SNWinFailure{S_OK, SN_DISABLED};
}
static std::wstring tag(const std::string &id) {
  std::wstring value = sn_wide(id);
  if (value.empty() || value.size() > 16)
    throw SNWinFailure{E_INVALIDARG, SN_INVALID};
  return value;
}
int64_t scoop_notification_post(void *pointer, int64_t request,
                                SNContent *content) {
  SNWindows *owner = static_cast<SNWindows *>(pointer);
  SNEventPtr event(sn_event(SN_POST, request));
  try {
    available(owner);
    const std::string id = sn_content_text(content, 0);
    SNHString identifier(tag(id));
    if (content->count > 5)
      throw SNWinFailure{E_INVALIDARG, SN_INVALID};
    SDCom<IInspectable> instance;
    SNHString xml_class(L"Windows.Data.Xml.Dom.XmlDocument");
    sn_check(RoActivateInstance(xml_class.value, instance.put()));
    SDCom<sn_xml::IXmlDocument> xml;
    sn_check(instance->QueryInterface(__uuidof(sn_xml::IXmlDocument),
                                      reinterpret_cast<void **>(xml.put())));
    SDCom<sn_xml::IXmlDocumentIO> io;
    sn_check(xml->QueryInterface(__uuidof(sn_xml::IXmlDocumentIO),
                                 reinterpret_cast<void **>(io.put())));
    SNHString text(sn_notification_xml(content));
    sn_check(io->LoadXml(text.value), SN_INVALID);
    SDCom<sn_toast::IToastNotificationFactory> factory;
    SNHString toast_class(L"Windows.UI.Notifications.ToastNotification");
    sn_check(RoGetActivationFactory(
        toast_class.value, __uuidof(sn_toast::IToastNotificationFactory),
        reinterpret_cast<void **>(factory.put())));
    auto toast = std::make_unique<SNToast>();
    toast->generation = owner->generation++;
    sn_check(factory->CreateToastNotification(xml.value, toast->value.put()));
    SDCom<sn_toast::IToastNotification2> tagged;
    sn_check(
        toast->value->QueryInterface(__uuidof(sn_toast::IToastNotification2),
                                     reinterpret_cast<void **>(tagged.put())));
    SNHString group(L"scoop");
    sn_check(tagged->put_Tag(identifier.value), SN_INVALID);
    sn_check(tagged->put_Group(group.value), SN_INVALID);
    sn_toast_handlers(owner, toast.get(), id);
    sn_check(owner->notifier->Show(toast->value.value));
    owner->notifications[id] = std::move(toast);
  } catch (const SNWinFailure &error) {
    sn_event_error(event.get(), error.status, static_cast<uint32_t>(error.code),
                   nullptr);
  } catch (const std::bad_alloc &) {
    sn_event_error(event.get(), SN_NATIVE_FAILURE,
                   static_cast<uint32_t>(E_OUTOFMEMORY), nullptr);
  }
  try {
    return static_cast<uint32_t>(owner->queue->push(event.release()));
  } catch (const std::bad_alloc &) {
    return static_cast<uint32_t>(E_OUTOFMEMORY);
  }
}
int64_t scoop_notification_withdraw(void *pointer, int64_t request,
                                    const uint8_t *id, int64_t length) {
  SNWindows *owner = static_cast<SNWindows *>(pointer);
  SNEventPtr event(sn_event(SN_WITHDRAW, request));
  try {
    if (!owner->configured)
      throw SNWinFailure{S_OK, SN_NOT_CONFIGURED};
    std::string key(reinterpret_cast<const char *>(id),
                    static_cast<size_t>(length));
    SNHString identifier(tag(key));
    if (!owner->history.value) {
      SDCom<sn_toast::IToastNotificationManagerStatics2> manager;
      SNHString name(L"Windows.UI.Notifications.ToastNotificationManager");
      sn_check(RoGetActivationFactory(
          name.value, __uuidof(sn_toast::IToastNotificationManagerStatics2),
          reinterpret_cast<void **>(manager.put())));
      sn_check(manager->get_History(owner->history.put()));
    }
    SNHString group(L"scoop"), application(owner->id);
    sn_check(owner->history->RemoveGroupedTagWithId(
        identifier.value, group.value, application.value));
    owner->notifications.erase(key);
  } catch (const SNWinFailure &error) {
    sn_event_error(event.get(), error.status, static_cast<uint32_t>(error.code),
                   nullptr);
  } catch (const std::bad_alloc &) {
    sn_event_error(event.get(), SN_NATIVE_FAILURE,
                   static_cast<uint32_t>(E_OUTOFMEMORY), nullptr);
  }
  try {
    return static_cast<uint32_t>(owner->queue->push(event.release()));
  } catch (const std::bad_alloc &) {
    return static_cast<uint32_t>(E_OUTOFMEMORY);
  }
}
