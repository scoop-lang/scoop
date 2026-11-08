#pragma once

#include "text.h"
#include "third_party/externals/harfbuzz/src/hb-ot.h"
#include "third_party/externals/harfbuzz/src/hb.h"
#include <climits>

using STHbFace = std::unique_ptr<hb_face_t, decltype(&hb_face_destroy)>;
STHbFace st_hb_face(const sk_sp<SkTypeface> &face);
std::string st_font_name(hb_face_t *face, hb_ot_name_id_t id);

struct STFontString {
  const char *data;
  int64_t length;
};
inline STFontString st_font_string(const std::string &text) {
  return {text.data(), static_cast<int64_t>(text.size())};
}
struct STFontInfo {
  STFontString style, postscript;
  int64_t family, collection_index;
  int32_t weight, width, slant, variable, valid;
};
struct STFontDescription {
  sk_sp<SkTypeface> face;
  std::string style, postscript;
  int64_t family = 0, collection_index = -1;
  int32_t axis_count = 0;
  void describe();
  STFontInfo info() const;
};
struct STCatalogFamily {
  std::string name;
  sk_sp<SkFontStyleSet> styles;
  std::vector<int64_t> faces;
};
struct STCatalogEntry {
  STFontDescription font;
  int style_index = 0;
};
struct STFontCatalog : STObject {
  std::vector<STCatalogFamily> families;
  std::vector<STCatalogEntry> faces;
  explicit STFontCatalog(const sk_sp<SkFontMgr> &system);
  STFontCatalog(const sk_sp<SkFontMgr> &system, sk_sp<SkData> bytes);
  STFontDescription &load(int64_t index);
};
struct STFontAxis {
  std::string tag, name;
  double minimum, default_value, maximum, current;
  bool hidden;
};
struct STNamedFont {
  std::string name, postscript;
  std::vector<float> coordinates;
};
struct STFontFace : STObject {
  STFontDescription font;
  std::vector<STFontAxis> axes;
  std::vector<STNamedFont> instances;
  explicit STFontFace(STFontDescription value);
};
struct STFontAxisInfo {
  STFontString tag, name;
  double minimum, default_value, maximum, current;
  int32_t hidden;
};
struct STNamedFontInfo {
  STFontString name, postscript;
};
struct STFontCoordinate {
  uint32_t tag;
  double value;
};
