#include "font_catalog.h"
#include "include/core/SkStream.h"

STFontFace::STFontFace(STFontDescription value) : font(std::move(value)) {
  auto hb = st_hb_face(font.face);
  unsigned count = static_cast<unsigned>(font.axis_count);
  std::vector<hb_ot_var_axis_info_t> information(count);
  hb_ot_var_get_axis_infos(hb.get(), 0, &count, information.data());
  if (count != static_cast<unsigned>(font.axis_count))
    throw std::runtime_error("font axes are unavailable");
  std::vector<SkFontArguments::VariationPosition::Coordinate> positions(count);
  if (count > 0 && font.face->getVariationDesignPosition(
                       {positions.data(), positions.size()}) != font.axis_count)
    throw std::runtime_error("font coordinates are unavailable");
  for (const auto &axis : information) {
    auto current = std::find_if(
        positions.begin(), positions.end(),
        [&](const auto &position) { return position.axis == axis.tag; });
    if (current == positions.end())
      throw std::runtime_error("font axis coordinate is unavailable");
    char tag[4];
    hb_tag_to_string(axis.tag, tag);
    axes.push_back({std::string(tag, 4), st_font_name(hb.get(), axis.name_id),
                    axis.min_value, axis.default_value, axis.max_value,
                    current->value,
                    (axis.flags & HB_OT_VAR_AXIS_FLAG_HIDDEN) != 0});
  }
  const auto total = hb_ot_var_get_named_instance_count(hb.get());
  for (unsigned index = 0; index < total; ++index) {
    STNamedFont instance;
    instance.name = st_font_name(
        hb.get(),
        hb_ot_var_named_instance_get_subfamily_name_id(hb.get(), index));
    instance.postscript = st_font_name(
        hb.get(),
        hb_ot_var_named_instance_get_postscript_name_id(hb.get(), index));
    instance.coordinates.resize(count);
    unsigned actual = count;
    const auto available = hb_ot_var_named_instance_get_design_coords(
        hb.get(), index, &actual, instance.coordinates.data());
    if (actual != count || available != count)
      throw std::runtime_error("named font coordinates are unavailable");
    instances.push_back(std::move(instance));
  }
}

extern "C" {
STFontInfo scoop_graphics_text_face_info(void *pointer) {
  return static_cast<STFontFace *>(pointer)->font.info();
}
int64_t scoop_graphics_text_face_axes(void *pointer) {
  return static_cast<STFontFace *>(pointer)->axes.size();
}
STFontAxisInfo scoop_graphics_text_face_axis(void *pointer, int64_t index) {
  const auto &axis = static_cast<STFontFace *>(pointer)->axes[index];
  return {st_font_string(axis.tag),
          st_font_string(axis.name),
          axis.minimum,
          axis.default_value,
          axis.maximum,
          axis.current,
          axis.hidden};
}
int64_t scoop_graphics_text_face_instances(void *pointer) {
  return static_cast<STFontFace *>(pointer)->instances.size();
}
STNamedFontInfo scoop_graphics_text_face_instance(void *pointer,
                                                  int64_t index) {
  const auto &value = static_cast<STFontFace *>(pointer)->instances[index];
  return {st_font_string(value.name), st_font_string(value.postscript)};
}
double scoop_graphics_text_face_instance_coordinate(void *pointer,
                                                    int64_t instance,
                                                    int64_t axis) {
  return static_cast<STFontFace *>(pointer)
      ->instances[instance]
      .coordinates[axis];
}
void *scoop_graphics_text_face_variations(void *pointer,
                                          const STFontCoordinate *values,
                                          int64_t count) {
  try {
    auto &source = *static_cast<STFontFace *>(pointer);
    std::vector<SkFontArguments::VariationPosition::Coordinate> coordinates;
    for (int64_t index = 0; index < count; ++index)
      coordinates.push_back(
          {values[index].tag, static_cast<float>(values[index].value)});
    SkFontArguments arguments;
#if defined(_WIN32)
    int collectionIndex = 0;
    auto stream = source.font.face->openStream(&collectionIndex);
    if (!stream)
      return nullptr;
    arguments.setCollectionIndex(collectionIndex);
#endif
    arguments.setVariationDesignPosition(
        {coordinates.data(), static_cast<int>(coordinates.size())});
    auto clone = source.font.face->makeClone(arguments);
    if (!clone)
      return nullptr;
    auto description = source.font;
    description.face = std::move(clone);
    description.describe();
    return new STFontFace(std::move(description));
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_text_fonts_faces(void *pointer, void *const *faces,
                                        int64_t count, const char *alias,
                                        int64_t length, bool replace) {
  try {
    auto &fonts = *static_cast<STFonts *>(pointer);
    SkString name(alias, static_cast<size_t>(length));
    auto next = fonts.entries;
    if (replace)
      next.erase(std::remove_if(
                     next.begin(), next.end(),
                     [&](const auto &entry) { return entry.family == name; }),
                 next.end());
    for (int64_t index = 0; index < count; ++index)
      next.push_back(
          {static_cast<STFontFace *>(faces[index])->font.face, name});
    fonts.entries.swap(next);
    return 1;
  } catch (...) {
    return 0;
  }
}
}
