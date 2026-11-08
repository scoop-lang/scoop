#include "../shared/font_catalog.h"
#include "include/ports/SkCFObject.h"
#include "include/ports/SkFontMgr_mac_ct.h"
#include <CoreText/CoreText.h>

sk_sp<SkFontMgr> st_system_fonts() { return SkFontMgr_New_CoreText(nullptr); }

std::vector<sk_sp<SkTypeface>> st_data_faces(const sk_sp<SkFontMgr> &system,
                                             const sk_sp<SkData> &data,
                                             int first, int count) {
  if (!data || data->size() > UINT_MAX || first < 0 || count <= 0)
    return {};
  using Blob = std::unique_ptr<hb_blob_t, decltype(&hb_blob_destroy)>;
  Blob bytes(hb_blob_create(static_cast<const char *>(data->data()),
                            static_cast<unsigned>(data->size()),
                            HB_MEMORY_MODE_READONLY, nullptr, nullptr),
             hb_blob_destroy);
  const auto faces = hb_face_count(bytes.get());
  if (static_cast<unsigned>(first) > faces ||
      static_cast<unsigned>(count) > faces - first)
    return {};
  std::vector<sk_sp<SkTypeface>> result;
  for (int index = 0; index < count; ++index) {
    sk_sp<SkData> source = data;
    if (first + index != 0) {
      STHbFace input(hb_face_create(bytes.get(), first + index),
                     hb_face_destroy);
      STHbFace builder(hb_face_builder_create(), hb_face_destroy);
      auto table_count =
          hb_face_get_table_tags(input.get(), 0, nullptr, nullptr);
      std::vector<hb_tag_t> tags(table_count);
      hb_face_get_table_tags(input.get(), 0, &table_count, tags.data());
      for (auto tag : tags) {
        // A signature cannot describe a reserialized standalone font.
        if (tag == HB_TAG('D', 'S', 'I', 'G'))
          continue;
        Blob table(hb_face_reference_table(input.get(), tag), hb_blob_destroy);
        if (!hb_face_builder_add_table(builder.get(), tag, table.get()))
          return {};
      }
      Blob serialized(hb_face_reference_blob(builder.get()), hb_blob_destroy);
      unsigned length = 0;
      const auto contents = hb_blob_get_data(serialized.get(), &length);
      if (!length)
        return {};
      source = SkData::MakeWithProc(
          contents, length,
          [](const void *, void *context) {
            hb_blob_destroy(static_cast<hb_blob_t *>(context));
          },
          serialized.release());
    }
    auto face = system->makeFromData(source, 0);
    if (!face)
      return {};
    result.push_back(std::move(face));
  }
  return result;
}

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
