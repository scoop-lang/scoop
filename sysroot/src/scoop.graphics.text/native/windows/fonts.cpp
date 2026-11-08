#include "../shared/text.h"
#include "include/ports/SkTypeface_win.h"

sk_sp<SkFontMgr> st_system_fonts() { return SkFontMgr_New_DirectWrite(); }

std::vector<sk_sp<SkTypeface>> st_data_faces(const sk_sp<SkFontMgr> &system,
                                             const sk_sp<SkData> &data,
                                             int first, int count) {
  std::vector<sk_sp<SkTypeface>> result;
  for (int index = 0; index < count; ++index) {
    auto face = system->makeFromData(data, first + index);
    if (!face)
      return {};
    result.push_back(std::move(face));
  }
  return result;
}

SkString st_system_family(int category) {
  const char *family;
  switch (category) {
  case 0:
    family = "Segoe UI";
    break;
  case 1:
    family = "Times New Roman";
    break;
  case 2:
    family = "Consolas";
    break;
  default:
    throw std::invalid_argument("invalid system font category");
  }
  auto manager = st_system_fonts();
  if (!manager)
    throw std::runtime_error("system font manager is unavailable");
  auto face = manager->legacyMakeTypeface(family, SkFontStyle());
  if (!face)
    throw std::runtime_error("system font is unavailable");
  SkString resolved;
  face->getFamilyName(&resolved);
  if (resolved.isEmpty())
    throw std::runtime_error("system font family is unavailable");
  return resolved;
}
