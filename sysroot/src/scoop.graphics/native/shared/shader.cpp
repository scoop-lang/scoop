#include "graphics.h"
#include "include/effects/SkImageFilters.h"
#include "include/effects/SkRuntimeEffect.h"
#include <algorithm>
#include <limits>
#include <string_view>
#include <vector>

namespace {
using Uniform = SkRuntimeEffect::Uniform;

struct ShaderProgram final : SGObject {
  SkRuntimeEffect::Result result;
  explicit ShaderProgram(SkString source)
      : result(SkRuntimeEffect::MakeForShader(std::move(source))) {
    if (result.effect) {
      for (const auto &child : result.effect->children()) {
        if (child.type != SkRuntimeEffect::ChildType::kShader) {
          result.errorText = SkString("expected shader input: ");
          result.errorText.append(child.name.data(), child.name.size());
          result.effect.reset();
          break;
        }
      }
    }
  }
};

struct Shader final : SGObject {
  sk_sp<GrDirectContext> context;
  sk_sp<SkShader> shader;
  Shader(sk_sp<SkShader> s, sk_sp<GrDirectContext> c,
         std::shared_ptr<SGGLState> state)
      : context(std::move(c)), shader(std::move(s)) {
    gl = std::move(state);
  }
};

struct Bindings final : SGObject {
  sk_sp<GrDirectContext> context;
  SkRuntimeEffectBuilder builder;
  std::vector<bool> gpuInputs;
  int source;
  Bindings(const ShaderProgram &program, int sourceIndex)
      : builder(program.result.effect),
        gpuInputs(program.result.effect->children().size()),
        source(sourceIndex) {}
};

struct Text {
  const char *data;
  int64_t length;
};
std::string_view nameView(const char *data, int64_t length) {
  return {data, static_cast<size_t>(length)};
}
} // namespace

extern "C" {
ShaderProgram *scoop_graphics_shader_compile(const char *text, int64_t length) {
  try {
    return new ShaderProgram(SkString(text, static_cast<size_t>(length)));
  } catch (...) {
    return nullptr;
  }
}
Text scoop_graphics_shader_diagnostic(ShaderProgram *program) {
  const auto &result = program->result;
  return result.effect ? Text{}
                       : Text{result.errorText.c_str(),
                              static_cast<int64_t>(result.errorText.size())};
}
int32_t scoop_graphics_shader_source(ShaderProgram *program, const char *name,
                                     int64_t length) {
  const auto *child = program->result.effect->findChild(nameView(name, length));
  return child ? child->index : -1;
}
Bindings *scoop_graphics_shader_bindings(ShaderProgram *program,
                                         int32_t source) {
  try {
    return new Bindings(*program, source);
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_shader_numbers(Bindings *bindings, const char *name,
                                      int64_t length, const void *values,
                                      int64_t count, bool integers,
                                      bool scalar) {
  auto uniform = bindings->builder.uniform(nameView(name, length));
  const auto *var = uniform.fVar;
  if (!var || count <= 0 || count > std::numeric_limits<int>::max() ||
      var->sizeInBytes() / sizeof(float) != static_cast<size_t>(count))
    return -1;
  const bool integerType =
      var->type >= Uniform::Type::kInt && var->type <= Uniform::Type::kInt4;
  if (integerType != integers ||
      (scalar &&
       (var->isArray() ||
        var->type != (integers ? Uniform::Type::kInt : Uniform::Type::kFloat))))
    return -1;
  try {
    return integers ? uniform.set(static_cast<const int32_t *>(values),
                                  static_cast<int>(count))
                    : uniform.set(static_cast<const float *>(values),
                                  static_cast<int>(count));
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_shader_color(Bindings *bindings, const char *name,
                                    int64_t length, SGColor color) {
  auto uniform = bindings->builder.uniform(nameView(name, length));
  const auto *var = uniform.fVar;
  if (!var || !var->isColor() || var->isArray())
    return -1;
  const float values[] = {
      static_cast<float>(color.red), static_cast<float>(color.green),
      static_cast<float>(color.blue), static_cast<float>(color.alpha)};
  try {
    return uniform.set(values, var->type == Uniform::Type::kFloat3 ? 3 : 4);
  } catch (...) {
    return 0;
  }
}
int32_t scoop_graphics_shader_child(Bindings *bindings, const char *name,
                                    int64_t length, Shader *shader) {
  auto slot = bindings->builder.child(nameView(name, length));
  if (!slot.fChild || slot.fChild->index == bindings->source)
    return -1;
  const size_t index = static_cast<size_t>(slot.fChild->index);
  if (shader->context && shader->context != bindings->context) {
    for (size_t i = 0; i < bindings->gpuInputs.size(); ++i)
      if (i != index && bindings->gpuInputs[i])
        return -1;
  }
  slot = shader->shader;
  bindings->gpuInputs[index] = static_cast<bool>(shader->context);
  if (shader->context) {
    bindings->context = shader->context;
    bindings->gl = shader->gl;
  } else if (std::none_of(bindings->gpuInputs.begin(),
                          bindings->gpuInputs.end(),
                          [](bool gpu) { return gpu; })) {
    bindings->context.reset();
    bindings->gl.reset();
  }
  return 1;
}
Text scoop_graphics_shader_missing(Bindings *bindings) {
  const auto bound = bindings->builder.children();
  for (const auto &child : bindings->builder.effect()->children()) {
    if (child.index != bindings->source && !bound[child.index].shader())
      return {child.name.data(), static_cast<int64_t>(child.name.size())};
  }
  return {};
}
Shader *scoop_graphics_shader_make(Bindings *bindings) {
  try {
    auto shader = bindings->builder.makeShader();
    return shader
               ? new Shader(std::move(shader), bindings->context, bindings->gl)
               : nullptr;
  } catch (...) {
    return nullptr;
  }
}
SGFilter *scoop_graphics_shader_filter(Bindings *bindings, double radius) {
  try {
    const auto &child =
        bindings->builder.effect()->children()[bindings->source];
    auto filter = SkImageFilters::RuntimeShader(
        bindings->builder, static_cast<float>(radius), child.name, nullptr);
    return filter ? new SGFilter(std::move(filter), bindings->context,
                                 bindings->gl)
                  : nullptr;
  } catch (...) {
    return nullptr;
  }
}
Shader *scoop_graphics_image_shader(SGImage *image, bool linear) {
  try {
    auto shader = image->image->makeShader(SkSamplingOptions(
        linear ? SkFilterMode::kLinear : SkFilterMode::kNearest));
    return shader ? new Shader(std::move(shader), image->context, image->gl)
                  : nullptr;
  } catch (...) {
    return nullptr;
  }
}
int32_t scoop_graphics_paint_shader(SGCanvas *canvas, SGPaint *paint,
                                    Shader *shader) {
  if (!sg_compatible(canvas, shader->context))
    return -1;
  paint->paint.setShader(shader->shader);
  return 1;
}
int32_t scoop_graphics_canvas_backdrop_mask(SGCanvas *canvas, SGRect bounds,
                                            SGFilter *filter, Shader *mask) {
  if (!sg_compatible(canvas, filter->context) ||
      !sg_compatible(canvas, mask->context))
    return -1;
  try {
    if (canvas->recording)
      canvas->recording->independentTiles = false;
    if (filter->filter) {
      SkAutoCanvasRestore state(canvas->canvas, true);
      canvas->canvas->clipRect(sg_rect(bounds));
      canvas->canvas->clipShader(mask->shader);
      SkPaint restore;
      restore.setBlendMode(SkBlendMode::kSrc);
      canvas->canvas->saveLayer(
          SkCanvas::SaveLayerRec(nullptr, &restore, filter->filter.get(), 0));
    }
    return 1;
  } catch (...) {
    return 0;
  }
}
}
