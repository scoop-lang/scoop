#include "../text_services.h"
#define WIN32_LEAN_AND_MEAN
#include <climits>
#include <memory>
#include <spellcheck.h>
#include <stdexcept>
#include <windows.h>

template <class T> struct ReleaseCom {
  void operator()(T *value) const {
    if (value)
      value->Release();
  }
};
template <class T> using ComOwner = std::unique_ptr<T, ReleaseCom<T>>;
struct FreeCom {
  void operator()(wchar_t *value) const { CoTaskMemFree(value); }
};

static std::wstring wide(const uint8_t *bytes, int64_t length) {
  if (!length)
    return {};
  if (length > INT_MAX)
    throw std::length_error("text check input");
  int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                                  reinterpret_cast<const char *>(bytes),
                                  static_cast<int>(length), nullptr, 0);
  if (!count)
    throw std::invalid_argument("UTF-8 text");
  std::wstring result(static_cast<size_t>(count), 0);
  MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                      reinterpret_cast<const char *>(bytes),
                      static_cast<int>(length), result.data(), count);
  return result;
}
static std::string utf8(const wchar_t *text, int length) {
  if (!length)
    return {};
  int count = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text, length,
                                  nullptr, 0, nullptr, nullptr);
  if (!count)
    throw std::invalid_argument("UTF-16 text");
  std::string result(static_cast<size_t>(count), 0);
  WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text, length,
                      result.data(), count, nullptr, nullptr);
  return result;
}
static ComOwner<ISpellCheckerFactory> factory() {
  ISpellCheckerFactory *value = nullptr;
  CoCreateInstance(__uuidof(SpellCheckerFactory), nullptr, CLSCTX_INPROC_SERVER,
                   __uuidof(ISpellCheckerFactory),
                   reinterpret_cast<void **>(&value));
  return ComOwner<ISpellCheckerFactory>(value);
}
extern "C" int32_t scoop_window_text_service_flags() {
  return factory() ? 5 : 0;
}

extern "C" SWTextServiceResult
scoop_window_text_check(const uint8_t *bytes, int64_t length, int32_t flags,
                        const uint8_t *language, int64_t language_length) {
  try {
    auto result = std::make_unique<SWTextServiceResults>();
    auto create = factory();
    if (!create || !(flags & 5))
      return {result.release(), 0};
    auto text = wide(bytes, length);
    auto locale = wide(language, language_length);
    if (locale.empty()) {
      wchar_t name[LOCALE_NAME_MAX_LENGTH] = {};
      if (!GetUserDefaultLocaleName(name, LOCALE_NAME_MAX_LENGTH))
        return {result.release(), 0};
      locale = name;
    }
    BOOL supported = FALSE;
    HRESULT status = create->IsSupported(locale.c_str(), &supported);
    if (FAILED(status))
      return {nullptr, status};
    if (!supported)
      return {result.release(), 0};
    ISpellChecker *raw_checker = nullptr;
    status = create->CreateSpellChecker(locale.c_str(), &raw_checker);
    ComOwner<ISpellChecker> checker(raw_checker);
    if (FAILED(status))
      return {nullptr, status};
    IEnumSpellingError *raw_errors = nullptr;
    status = checker->ComprehensiveCheck(text.c_str(), &raw_errors);
    ComOwner<IEnumSpellingError> errors(raw_errors);
    if (FAILED(status))
      return {nullptr, status};
    for (;;) {
      ISpellingError *raw_error = nullptr;
      status = errors->Next(&raw_error);
      ComOwner<ISpellingError> error(raw_error);
      if (status == S_FALSE)
        break;
      if (FAILED(status))
        return {nullptr, status};
      ULONG start = 0, count = 0;
      CORRECTIVE_ACTION action = CORRECTIVE_ACTION_NONE;
      if (FAILED(status = error->get_StartIndex(&start)) ||
          FAILED(status = error->get_Length(&count)) ||
          FAILED(status = error->get_CorrectiveAction(&action)))
        return {nullptr, status};
      if (start > text.size() || count > text.size() - start ||
          action == CORRECTIVE_ACTION_NONE)
        continue;
      SWTextServiceRow row;
      row.start = static_cast<int64_t>(
          utf8(text.data(), static_cast<int>(start)).size());
      row.end = row.start +
                static_cast<int64_t>(
                    utf8(text.data() + start, static_cast<int>(count)).size());
      row.kind = 1;
      if (action == CORRECTIVE_ACTION_GET_SUGGESTIONS) {
        IEnumString *raw_suggestions = nullptr;
        status = checker->Suggest(text.substr(start, count).c_str(),
                                  &raw_suggestions);
        ComOwner<IEnumString> suggestions(raw_suggestions);
        if (FAILED(status))
          return {nullptr, status};
        for (;;) {
          wchar_t *raw = nullptr;
          ULONG fetched = 0;
          status = suggestions->Next(1, &raw, &fetched);
          std::unique_ptr<wchar_t, FreeCom> suggestion(raw);
          if (FAILED(status))
            return {nullptr, status};
          if (!fetched)
            break;
          row.suggestions.push_back(utf8(raw, static_cast<int>(wcslen(raw))));
        }
      } else if (action == CORRECTIVE_ACTION_REPLACE) {
        wchar_t *raw = nullptr;
        status = error->get_Replacement(&raw);
        std::unique_ptr<wchar_t, FreeCom> replacement(raw);
        if (FAILED(status))
          return {nullptr, status};
        row.suggestions.push_back(utf8(raw, static_cast<int>(wcslen(raw))));
        if (flags & 4)
          row.kind = 3;
      } else if (action == CORRECTIVE_ACTION_DELETE) {
        row.suggestions.emplace_back();
        if (flags & 4)
          row.kind = 3;
      }
      if ((row.kind == 1 && (flags & 1)) || row.kind == 3)
        result->rows.push_back(std::move(row));
    }
    return {result.release(), 0};
  } catch (const std::bad_alloc &) {
    return {nullptr, ERROR_NOT_ENOUGH_MEMORY};
  } catch (const std::length_error &) {
    return {nullptr, ERROR_ARITHMETIC_OVERFLOW};
  } catch (const std::invalid_argument &) {
    return {nullptr, ERROR_NO_UNICODE_TRANSLATION};
  }
}

extern "C" SWTextServiceResult scoop_window_text_services(void *) {
  auto *value = new (std::nothrow) SWTextServiceResults;
  return {value, value ? 0 : ERROR_NOT_ENOUGH_MEMORY};
}
extern "C" SWTextServiceOutput
scoop_window_text_service_perform(const uint8_t *, int64_t, const uint8_t *,
                                  int64_t) {
  return {};
}
extern "C" int64_t scoop_window_text_definition(void *, const uint8_t *,
                                                int64_t, double, double, double,
                                                double) {
  return 0;
}
