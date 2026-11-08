#include "font_catalog.h"

STHbFace st_hb_face(const sk_sp<SkTypeface> &face) {
  auto retained = std::make_unique<sk_sp<SkTypeface>>(face);
  auto result = hb_face_create_for_tables(
      [](hb_face_t *, hb_tag_t tag, void *context) -> hb_blob_t * {
        auto &typeface = *static_cast<sk_sp<SkTypeface> *>(context);
        auto data = typeface->copyTableData(tag);
        if (!data || data->size() > UINT_MAX)
          return hb_blob_get_empty();
        auto bytes = static_cast<const char *>(data->data());
        auto size = static_cast<unsigned>(data->size());
        return hb_blob_create(
            bytes, size, HB_MEMORY_MODE_READONLY, data.release(),
            [](void *pointer) { static_cast<SkData *>(pointer)->unref(); });
      },
      retained.release(),
      [](void *pointer) { delete static_cast<sk_sp<SkTypeface> *>(pointer); });
  return STHbFace(result, hb_face_destroy);
}

std::string st_font_name(hb_face_t *face, hb_ot_name_id_t id) {
  if (id == HB_OT_NAME_ID_INVALID)
    return {};
  auto language = hb_language_get_default();
  unsigned count = 0;
  auto length = hb_ot_name_get_utf8(face, id, language, &count, nullptr);
  if (!length) {
    language = hb_language_from_string("en", -1);
    length = hb_ot_name_get_utf8(face, id, language, &count, nullptr);
  }
  if (!length)
    return {};
  std::string name(length + 1, '\0');
  count = static_cast<unsigned>(name.size());
  hb_ot_name_get_utf8(face, id, language, &count, name.data());
  name.resize(count);
  return name;
}

void STFontDescription::describe() {
  auto hb = st_hb_face(face);
  const auto native_count = face->getVariationDesignParameters({});
  axis_count = static_cast<int>(hb_ot_var_get_axis_count(hb.get()));
  if ((axis_count > 0 || native_count > 0) && axis_count != native_count)
    throw std::runtime_error("font variation metadata is unavailable");
  if (style.empty())
    style = st_font_name(hb.get(), HB_OT_NAME_ID_TYPOGRAPHIC_SUBFAMILY);
  if (style.empty())
    style = st_font_name(hb.get(), HB_OT_NAME_ID_FONT_SUBFAMILY);
  SkString name;
  postscript = face->getPostScriptName(&name) ? name.c_str() : "";
}

STFontInfo STFontDescription::info() const {
  const auto value = face->fontStyle();
  return {st_font_string(style),
          st_font_string(postscript),
          family,
          collection_index,
          value.weight(),
          value.width(),
          value.slant(),
          axis_count > 0,
          1};
}

STFontCatalog::STFontCatalog(const sk_sp<SkFontMgr> &system) {
  for (int index = 0; index < system->countFamilies(); ++index) {
    SkString name;
    system->getFamilyName(index, &name);
    auto styles = system->createStyleSet(index);
    if (!styles)
      throw std::runtime_error("system font style set is unavailable");
    STCatalogFamily family{name.c_str(), std::move(styles), {}};
    for (int style = 0; style < family.styles->count(); ++style) {
      STCatalogEntry entry;
      entry.font.family = static_cast<int64_t>(families.size());
      entry.style_index = style;
      family.faces.push_back(static_cast<int64_t>(faces.size()));
      faces.push_back(std::move(entry));
    }
    families.push_back(std::move(family));
  }
}

STFontCatalog::STFontCatalog(const sk_sp<SkFontMgr> &system,
                             sk_sp<SkData> bytes) {
  if (bytes->size() > UINT_MAX)
    throw std::runtime_error("font data exceeds HarfBuzz input size");
  std::unique_ptr<hb_blob_t, decltype(&hb_blob_destroy)> blob(
      hb_blob_create(static_cast<const char *>(bytes->data()),
                     static_cast<unsigned>(bytes->size()),
                     HB_MEMORY_MODE_READONLY, nullptr, nullptr),
      hb_blob_destroy);
  const auto count = hb_face_count(blob.get());
  if (!count || count > INT_MAX)
    throw std::runtime_error("invalid font file or collection");
  auto loaded = st_data_faces(system, bytes, 0, static_cast<int>(count));
  if (loaded.size() != count)
    throw std::runtime_error("font collection faces are unavailable");
  for (unsigned index = 0; index < count; ++index) {
    STCatalogEntry entry;
    entry.font.face = std::move(loaded[index]);
    if (!entry.font.face)
      throw std::runtime_error("font collection face is unavailable");
    entry.font.collection_index = index;
    entry.font.describe();
    SkString name;
    entry.font.face->getFamilyName(&name);
    auto found =
        std::find_if(families.begin(), families.end(), [&](const auto &family) {
          return family.name == name.c_str();
        });
    entry.font.family = std::distance(families.begin(), found);
    if (found == families.end())
      families.push_back({name.c_str(), nullptr, {}});
    families[entry.font.family].faces.push_back(
        static_cast<int64_t>(faces.size()));
    faces.push_back(std::move(entry));
  }
}

STFontDescription &STFontCatalog::load(int64_t index) {
  auto &entry = faces.at(static_cast<size_t>(index));
  if (!entry.font.face) {
    auto &styles = families[entry.font.family].styles;
    STFontDescription next;
    next.family = entry.font.family;
    SkFontStyle style;
    SkString name;
    styles->getStyle(entry.style_index, &style, &name);
    next.style = name.c_str();
    next.face = styles->createTypeface(entry.style_index);
    if (!next.face)
      throw std::runtime_error("system font face is unavailable");
    next.describe();
    entry.font = std::move(next);
  }
  return entry.font;
}

extern "C" {
void *scoop_graphics_text_catalog_system(void *pointer) {
  try {
    return new STFontCatalog(static_cast<STFonts *>(pointer)->system);
  } catch (...) {
    return nullptr;
  }
}
void *scoop_graphics_text_catalog_data(const void *bytes, int64_t length) {
  try {
    if (length <= 0 || static_cast<uint64_t>(length) > UINT_MAX)
      return nullptr;
    auto system = st_system_fonts();
    if (!system)
      return nullptr;
    auto data = SkData::MakeWithCopy(bytes, length);
    return data ? new STFontCatalog(system, std::move(data)) : nullptr;
  } catch (...) {
    return nullptr;
  }
}
int64_t scoop_graphics_text_catalog_families(void *pointer) {
  return static_cast<STFontCatalog *>(pointer)->families.size();
}
STFontString scoop_graphics_text_catalog_family(void *pointer, int64_t family) {
  return st_font_string(
      static_cast<STFontCatalog *>(pointer)->families[family].name);
}
int64_t scoop_graphics_text_catalog_faces(void *pointer, int64_t family) {
  return static_cast<STFontCatalog *>(pointer)->families[family].faces.size();
}
int64_t scoop_graphics_text_catalog_face_id(void *pointer, int64_t family,
                                            int64_t index) {
  return static_cast<STFontCatalog *>(pointer)->families[family].faces[index];
}
STFontInfo scoop_graphics_text_catalog_face_info(void *pointer, int64_t index) {
  try {
    return static_cast<STFontCatalog *>(pointer)->load(index).info();
  } catch (...) {
    return {};
  }
}
void *scoop_graphics_text_catalog_open(void *pointer, int64_t index) {
  try {
    return new STFontFace(static_cast<STFontCatalog *>(pointer)->load(index));
  } catch (...) {
    return nullptr;
  }
}
}
