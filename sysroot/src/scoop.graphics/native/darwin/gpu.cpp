#include "../shared/gpu.h"
#include "include/gpu/ganesh/mtl/GrMtlBackendSurface.h"

extern "C" bool scoop_graphics_metal_format_supported(uint64_t);
extern "C" void *scoop_graphics_metal_completion(const void *, const void *);
extern "C" bool scoop_graphics_metal_completion_submit(void *);
extern "C" void scoop_graphics_metal_completion_drop(void *);
extern "C" bool scoop_graphics_metal_queue_drain(const void *);

namespace {
struct MetalCompletion final : SGCompletion {
  void *command;
  explicit MetalCompletion(void *value) : command(value) {}
  ~MetalCompletion() override { scoop_graphics_metal_completion_drop(command); }
  bool submit() override {
    std::unique_ptr<MetalCompletion> self(this);
    return scoop_graphics_metal_completion_submit(command);
  }
};
} // namespace
std::unique_ptr<SGCompletion>
sg_gpu_completion(const SGDeviceInfo &device, const GrBackendTexture &texture) {
  auto result = std::make_unique<MetalCompletion>(nullptr);
  result->command = scoop_graphics_metal_completion(
      device.second, sg_gpu_texture(texture).pointer);
  return result->command ? std::move(result) : nullptr;
}
bool sg_gpu_drain_native(const SGDeviceInfo &device) {
  return scoop_graphics_metal_queue_drain(device.second);
}

extern "C" bool scoop_graphics_metal_texture_matches(const void *, const void *,
                                                     int32_t, int32_t,
                                                     uint64_t);
GrBackendFormat sg_gpu_format(uint64_t format) {
  return scoop_graphics_metal_format_supported(format)
             ? GrBackendFormats::MakeMtl(format)
             : GrBackendFormat{};
}
bool sg_gpu_srgb(uint64_t format) { return format == 71; }
GrBackendTexture sg_gpu_import(const SGDeviceInfo &device,
                               SGTextureInfo texture, int width, int height,
                               uint64_t format) {
  if (!scoop_graphics_metal_texture_matches(device.first, texture.pointer,
                                            width, height, format))
    return {};
  GrMtlTextureInfo info;
  info.fTexture = sk_ret_cfp(texture.pointer);
  return GrBackendTextures::MakeMtl(width, height, skgpu::Mipmapped::kNo, info);
}
SGTextureInfo sg_gpu_texture(const GrBackendTexture &texture) {
  GrMtlTextureInfo info;
  return GrBackendTextures::GetMtlTextureInfo(texture, &info)
             ? SGTextureInfo{info.fTexture.get(), 0, 0}
             : SGTextureInfo{};
}
