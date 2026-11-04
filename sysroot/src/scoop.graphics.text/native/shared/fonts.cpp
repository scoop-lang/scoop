#include "text.h"

STFonts::STFonts(sk_sp<SkFontMgr> manager, bool enabled)
    : system(std::move(manager)), fallback(enabled) {
  if (!fallback)
    return;
  static const char *aliases[] = {"sans-serif", "serif", "monospace"};
  for (int category = 0; category < 3; ++category) {
    auto family = st_system_family(category);
    auto styles = system->matchFamily(family.c_str());
    if (styles->count() == 0)
      throw std::runtime_error("system generic font is unavailable");
    for (int index = 0; index < styles->count(); ++index) {
      auto face = styles->createTypeface(index);
      if (!face)
        throw std::runtime_error("system generic font style is unavailable");
      system_aliases.push_back({std::move(face), SkString(aliases[category])});
    }
  }
}

sk_sp<sgt::FontCollection> STFonts::snapshot() const {
  auto result = sk_make_sp<sgt::FontCollection>();
  auto assets = sk_make_sp<sgt::TypefaceFontProvider>();
  for (const auto &entry : entries)
    assets->registerTypeface(entry.face, entry.family);
  for (const auto &entry : system_aliases) {
    const bool explicit_family = std::any_of(
        entries.begin(), entries.end(),
        [&](const STFontEntry &value) { return value.family == entry.family; });
    if (!explicit_family)
      assets->registerTypeface(entry.face, entry.family);
  }
  result->setAssetFontManager(assets);
  if (fallback)
    result->setDefaultFontManager(system);
  else
    result->disableFontFallback();
  return result;
}

extern "C" {
void *scoop_graphics_text_fonts_create(bool fallback) {
  try {
    auto manager = st_system_fonts();
    return manager ? new STFonts(std::move(manager), fallback) : nullptr;
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_text_fonts_add(void *pointer, const void *bytes,
                                      int64_t length, const char *alias,
                                      int64_t alias_length, int32_t index) {
  try {
    auto fonts = static_cast<STFonts *>(pointer);
    auto data = SkData::MakeWithCopy(bytes, (size_t)length);
    auto face = fonts->system->makeFromData(std::move(data), index);
    if (!face)
      return -1;
    fonts->entries.push_back(
        {std::move(face), SkString(alias, (size_t)alias_length)});
    return 1;
  } catch (...) {
    return 0;
  }
}
}
