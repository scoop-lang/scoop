#include "../shared/text.h"
#include "include/ports/SkCFObject.h"
#include "include/ports/SkFontMgr_mac_ct.h"
#include <CoreText/CoreText.h>

sk_sp<SkFontMgr> st_system_fonts() { return SkFontMgr_New_CoreText(nullptr); }

SkString st_system_family(int category) {
  sk_cfp<CTFontRef> font;
  switch (category) {
  case 0:
    font.reset(CTFontCreateUIFontForLanguage(kCTFontUIFontSystem, 0, nullptr));
    break;
  case 1:
    font.reset(CTFontCreateWithName(CFSTR("Times"), 0, nullptr));
    break;
  case 2:
    font.reset(
        CTFontCreateUIFontForLanguage(kCTFontUIFontUserFixedPitch, 0, nullptr));
    break;
  default:
    throw std::invalid_argument("invalid system font category");
  }
  if (!font)
    throw std::runtime_error("system font is unavailable");
  sk_cfp<CFStringRef> family(CTFontCopyFamilyName(font.get()));
  if (!family)
    throw std::runtime_error("system font family is unavailable");
  const CFIndex capacity =
      CFStringGetMaximumSizeForEncoding(CFStringGetLength(family.get()),
                                        kCFStringEncodingUTF8) +
      1;
  std::vector<char> text((size_t)capacity);
  if (!CFStringGetCString(family.get(), text.data(), capacity,
                          kCFStringEncodingUTF8))
    throw std::runtime_error("system font family is not Unicode");
  return SkString(text.data());
}
