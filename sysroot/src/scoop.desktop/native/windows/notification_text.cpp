#include "notification.h"
#include <limits.h>
#include <shellapi.h>

std::wstring sn_wide(const std::string &text) {
  if (text.size() > INT_MAX)
    throw SNWinFailure{E_INVALIDARG, SN_INVALID};
  std::wstring result;
  if (!sd_wide(reinterpret_cast<const uint8_t *>(text.data()),
               static_cast<int64_t>(text.size()), result))
    throw SNWinFailure{E_INVALIDARG, SN_INVALID};
  return result;
}
std::string sn_narrow(const std::wstring &text) {
  if (text.empty())
    return {};
  if (text.size() > INT_MAX)
    throw SNWinFailure{E_INVALIDARG, SN_INVALID};
  int length = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                                   static_cast<int>(text.size()), nullptr, 0,
                                   nullptr, nullptr);
  if (!length)
    throw SNWinFailure{E_INVALIDARG, SN_INVALID};
  std::string result(length, '\0');
  if (!WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(),
                           static_cast<int>(text.size()), result.data(), length,
                           nullptr, nullptr))
    throw SNWinFailure{E_INVALIDARG, SN_INVALID};
  return result;
}
static std::string escape(const std::string &text) {
  std::string result;
  for (char value : text) {
    switch (value) {
    case '&':
      result += "&amp;";
      break;
    case '<':
      result += "&lt;";
      break;
    case '>':
      result += "&gt;";
      break;
    case '\"':
      result += "&quot;";
      break;
    case '\'':
      result += "&apos;";
      break;
    default:
      result += value;
      break;
    }
  }
  return result;
}
static std::string encode(const std::string &text) {
  static const char hex[] = "0123456789ABCDEF";
  std::string result;
  for (unsigned char value : text) {
    if ((value >= 'a' && value <= 'z') || (value >= 'A' && value <= 'Z') ||
        (value >= '0' && value <= '9') || value == '-' || value == '_' ||
        value == '.' || value == '~')
      result += value;
    else {
      result += '%';
      result += hex[value >> 4];
      result += hex[value & 15];
    }
  }
  return result;
}
static std::string arguments(const SNContent *content, const char *action) {
  return "scoop=1&id=" + encode(sn_content_text(content, 0)) +
         "&action=" + encode(action) +
         "&payload=" + encode(sn_content_text(content, 3));
}
std::wstring sn_notification_xml(const SNContent *content) {
  std::string xml = "<toast activationType=\"foreground\" launch=\"" +
                    escape(arguments(content, "")) +
                    "\"><visual><binding template=\"ToastGeneric\"><text>" +
                    escape(sn_content_text(content, 1)) + "</text><text>" +
                    escape(sn_content_text(content, 2)) +
                    "</text></binding></visual>";
  if (content->count) {
    xml += "<actions>";
    for (size_t index = 0; index < content->count; ++index) {
      const SNAction &action = content->actions[index];
      xml += "<action activationType=\"foreground\" content=\"" +
             escape(action.title) + "\" arguments=\"" +
             escape(arguments(content, action.id)) + "\"/>";
    }
    xml += "</actions>";
  }
  if (content->silent)
    xml += "<audio silent=\"true\"/>";
  xml += "</toast>";
  return sn_wide(xml);
}
static int hex(char value) {
  if (value >= '0' && value <= '9')
    return value - '0';
  if (value >= 'a' && value <= 'f')
    return value - 'a' + 10;
  if (value >= 'A' && value <= 'F')
    return value - 'A' + 10;
  return -1;
}
static bool decode(const std::string &text, std::string &result) {
  result.clear();
  for (size_t index = 0; index < text.size(); ++index) {
    char value = text[index];
    if (value == '%') {
      if (index + 2 >= text.size())
        return false;
      int a = hex(text[index + 1]), b = hex(text[index + 2]);
      if (a < 0 || b < 0)
        return false;
      value = static_cast<char>((a << 4) | b);
      index += 2;
    }
    if (!value)
      return false;
    result += value;
  }
  sn_wide(result);
  return true;
}
bool sn_activation(const std::wstring &value, std::string &id,
                   std::string &action, std::string &payload) {
  std::string text = sn_narrow(value);
  const std::string prefix = "scoop=1&id=";
  if (!text.starts_with(prefix))
    return false;
  size_t a = text.find("&action=", prefix.size());
  size_t p = a == std::string::npos ? a : text.find("&payload=", a + 8);
  if (a == std::string::npos || p == std::string::npos)
    return false;
  return decode(text.substr(prefix.size(), a - prefix.size()), id) &&
         !id.empty() && decode(text.substr(a + 8, p - a - 8), action) &&
         decode(text.substr(p + 9), payload);
}
static std::wstring registry(HKEY hive, const std::wstring &path,
                             const wchar_t *name) {
  DWORD bytes = 0;
  constexpr DWORD flags = RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY;
  if (RegGetValueW(hive, path.c_str(), name, flags, nullptr, nullptr, &bytes) !=
          ERROR_SUCCESS ||
      bytes < sizeof(wchar_t))
    return {};
  std::wstring result(bytes / sizeof(wchar_t), L'\0');
  if (RegGetValueW(hive, path.c_str(), name, flags, nullptr, result.data(),
                   &bytes) != ERROR_SUCCESS)
    return {};
  result.resize(wcslen(result.c_str()));
  return result;
}
bool sn_configured(const std::wstring &id, CLSID &clsid) {
  wchar_t executable[32768];
  DWORD length = GetModuleFileNameW(nullptr, executable, 32768);
  if (!length || length >= 32768)
    return false;
  for (HKEY hive : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
    auto activator = registry(hive, L"Software\\Classes\\AppUserModelId\\" + id,
                              L"CustomActivator");
    if (activator.empty() || FAILED(CLSIDFromString(activator.c_str(), &clsid)))
      continue;
    auto server = registry(
        hive, L"Software\\Classes\\CLSID\\" + activator + L"\\LocalServer32",
        nullptr);
    if (server.empty())
      continue;
    int count = 0;
    LPWSTR *arguments = CommandLineToArgvW(server.c_str(), &count);
    bool matches =
        arguments && count > 0 && !_wcsicmp(arguments[0], executable);
    if (arguments)
      LocalFree(arguments);
    if (matches)
      return true;
  }
  return false;
}
