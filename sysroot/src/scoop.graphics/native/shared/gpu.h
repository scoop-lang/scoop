#pragma once

#include "graphics.h"
#include "include/gpu/ganesh/GrBackendSurface.h"

struct SGTextureInfo {
  const void *pointer;
  uint32_t name, state;
};
struct SGImageOwner {
  void *payload;
  void (*retain)(void *);
  void (*release)(void *, uint32_t);
};

// Prepared before the client callback. submit() consumes this native-only
// object.
struct SGCompletion {
  virtual ~SGCompletion() = default;
  virtual bool submit() = 0;
};
std::unique_ptr<SGCompletion> sg_gpu_completion(const SGDeviceInfo &,
                                                const GrBackendTexture &);
bool sg_gpu_drain_native(const SGDeviceInfo &);

GrBackendFormat sg_gpu_format(uint64_t format);
bool sg_gpu_srgb(uint64_t format);
GrBackendTexture sg_gpu_import(const SGDeviceInfo &, SGTextureInfo, int width,
                               int height, uint64_t format);
SGTextureInfo sg_gpu_texture(const GrBackendTexture &);
