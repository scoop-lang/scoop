#include "../shared/gpu.h"
#include "include/gpu/ganesh/gl/GrGLBackendSurface.h"
#include <GLES3/gl3.h>

namespace {
struct GLCompletion final : SGCompletion {
  bool submit() override {
    // GL command submission and deletion already retain objects until their
    // last GPU use.
    std::unique_ptr<GLCompletion> self(this);
    glFlush();
    return true;
  }
};
} // namespace
std::unique_ptr<SGCompletion> sg_gpu_completion(const SGDeviceInfo &,
                                                const GrBackendTexture &) {
  return std::make_unique<GLCompletion>();
}
bool sg_gpu_drain_native(const SGDeviceInfo &) {
  glFinish();
  return true;
}

GrBackendFormat sg_gpu_format(uint64_t format) {
  return GrBackendFormats::MakeGL(static_cast<GrGLenum>(format));
}
bool sg_gpu_srgb(uint64_t format) { return format == GL_SRGB8_ALPHA8; }
GrBackendTexture sg_gpu_import(const SGDeviceInfo &, SGTextureInfo texture,
                               int width, int height, uint64_t format) {
  if (!glIsTexture(texture.name))
    return {};
  GrGLTextureInfo info = {GL_TEXTURE_2D, texture.name,
                          static_cast<GrGLenum>(format)};
  return GrBackendTextures::MakeGL(width, height, skgpu::Mipmapped::kNo, info);
}
SGTextureInfo sg_gpu_texture(const GrBackendTexture &texture) {
  GrGLTextureInfo info;
  return GrBackendTextures::GetGLTextureInfo(texture, &info)
             ? SGTextureInfo{nullptr, info.fID, 0}
             : SGTextureInfo{};
}
