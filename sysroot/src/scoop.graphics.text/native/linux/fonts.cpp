#include "../shared/text.h"
#include "include/ports/SkFontMgr_fontconfig.h"
#include "include/ports/SkFontScanner_FreeType.h"
#include <fontconfig/fontconfig.h>

sk_sp<SkFontMgr> st_system_fonts() {
  return SkFontMgr_New_FontConfig(nullptr, SkFontScanner_Make_FreeType());
}

SkString st_system_family(int category) {
  static const char *families[] = {"sans-serif", "serif", "monospace"};
  if (category < 0 || category >= 3)
    throw std::invalid_argument("invalid system font category");
  using Pattern = std::unique_ptr<FcPattern, decltype(&FcPatternDestroy)>;
  Pattern pattern(
      FcNameParse(reinterpret_cast<const FcChar8 *>(families[category])),
      FcPatternDestroy);
  if (!pattern)
    throw std::bad_alloc();
  if (!FcConfigSubstitute(nullptr, pattern.get(), FcMatchPattern))
    throw std::runtime_error("system font configuration is unavailable");
  FcDefaultSubstitute(pattern.get());
  FcResult status;
  Pattern match(FcFontMatch(nullptr, pattern.get(), &status), FcPatternDestroy);
  FcChar8 *family = nullptr;
  if (!match ||
      FcPatternGetString(match.get(), FC_FAMILY, 0, &family) != FcResultMatch)
    throw std::runtime_error("system font family is unavailable");
  return SkString(reinterpret_cast<const char *>(family));
}
