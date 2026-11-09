#include "gpu.h"
#include "include/gpu/ganesh/GrContextThreadSafeProxy.h"
#include "include/gpu/ganesh/SkImageGanesh.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"
#include "include/private/chromium/GrSurfaceCharacterization.h"

namespace {
SkColorType color_type(int32_t layout, uint64_t format) {
  switch (layout) {
  case 0:
    return sg_gpu_srgb(format) ? kSRGBA_8888_SkColorType
                               : kRGBA_8888_SkColorType;
  case 1:
    return kBGRA_8888_SkColorType;
  case 2:
    return kRGBA_F16_SkColorType;
  case 3:
    return kRGBA_1010102_SkColorType;
  case 4:
    return kR8_unorm_SkColorType;
  case 5:
    return kR8G8_unorm_SkColorType;
  default:
    return kUnknown_SkColorType;
  }
}
sk_sp<SkColorSpace> color_space(bool linear) {
  return linear ? SkColorSpace::MakeSRGBLinear() : SkColorSpace::MakeSRGB();
}
struct BufferStorage {
  sk_sp<SkSurface> surface;
  explicit BufferStorage(sk_sp<SkSurface> value) : surface(std::move(value)) {}
};
void release_storage(void *pointer) {
  delete static_cast<std::shared_ptr<BufferStorage> *>(pointer);
}
struct NativeBuffer final : SGTarget {
  GrSurfaceCharacterization description;
  std::shared_ptr<BufferStorage> storage;
  NativeBuffer(GrSurfaceCharacterization desc, sk_sp<SkSurface> value)
      : description(std::move(desc)),
        storage(std::make_shared<BufferStorage>(std::move(value))) {
    surface = storage->surface;
    external = false;
  }
  bool prepareWrite() override {
    if (storage.use_count() == 1)
      return true;
    // Ganesh's implicit COW selects its default format. Preserve the requested
    // format.
    auto next = SkSurfaces::RenderTarget(context.get(), description,
                                         skgpu::Budgeted::kYes);
    if (!next)
      return false;
    SkPaint paint;
    paint.setBlendMode(SkBlendMode::kSrc);
    surface->draw(next->getCanvas(), 0, 0, SkSamplingOptions{}, &paint);
    auto replacement = std::make_shared<BufferStorage>(std::move(next));
    storage = std::move(replacement);
    surface = storage->surface;
    return true;
  }
  sk_sp<SkImage> snapshot() override {
    auto texture = SkSurfaces::GetBackendTexture(
        surface.get(), SkSurfaces::BackendHandleAccess::kFlushRead);
    if (!texture.isValid() || !context->submit(GrSyncCpu::kNo))
      return nullptr;
    auto info = surface->imageInfo();
    auto lease = std::make_unique<std::shared_ptr<BufferStorage>>(storage);
    return SkImages::BorrowTextureFrom(
        context.get(), texture, kTopLeft_GrSurfaceOrigin, info.colorType(),
        info.alphaType(), info.refColorSpace(), release_storage,
        lease.release());
  }
};
struct NativeAccess final : SGObject {
  sk_sp<GrDirectContext> context;
  sk_sp<SkSurface> surface;
  GrBackendTexture texture;
  SGDeviceInfo device;
  std::unique_ptr<SGCompletion> completion;
  bool ended = false;
  ~NativeAccess() override {
    if (!ended)
      context->resetContext();
  }
};
struct ImportedTexture {
  SGImageOwner owner;
  GrBackendTexture texture;
  uint32_t initialState;
  ImportedTexture(SGImageOwner value, uint32_t state)
      : owner(value), initialState(state) {
    owner.retain(owner.payload);
  }
  ~ImportedTexture() {
    owner.release(owner.payload, texture.isValid()
                                     ? sg_gpu_texture(texture).state
                                     : initialState);
  }
};
void release_texture(void *pointer) {
  delete static_cast<ImportedTexture *>(pointer);
}
} // namespace

extern "C" {
bool scoop_graphics_canvas_has_gpu(SGCanvas *canvas) {
  return bool(canvas->context);
}
SGContext *scoop_graphics_canvas_gpu(SGCanvas *canvas) {
  try {
    auto result = std::make_unique<SGContext>();
    result->context = canvas->context;
    result->gl = canvas->gl;
    result->device = canvas->device;
    return result->context && !result->context->abandoned() ? result.release()
                                                            : nullptr;
  } catch (...) {
    return nullptr;
  }
}
SGDeviceInfo scoop_graphics_gpu_info(SGContext *value) { return value->device; }
int32_t scoop_graphics_gpu_drain(SGContext *value) {
  try {
    return value->context->flushAndSubmit(GrSyncCpu::kYes).fSuccess &&
           sg_gpu_drain_native(value->device);
  } catch (...) {
    return 0;
  }
}
SGTarget *scoop_graphics_gpu_buffer(SGContext *device, int32_t width,
                                    int32_t height, uint64_t format,
                                    int32_t layout, bool linear) {
  try {
    auto info = SkImageInfo::Make(width, height, color_type(layout, format),
                                  layout >= 4 ? kOpaque_SkAlphaType
                                              : kPremul_SkAlphaType,
                                  color_space(linear));
    auto backend = sg_gpu_format(format);
    if (!backend.isValid())
      return nullptr;
    auto characterization =
        device->context->threadSafeProxy()->createCharacterization(
            device->context->getResourceCacheLimit(), info, backend, 1,
            kTopLeft_GrSurfaceOrigin, SkSurfaceProps{}, skgpu::Mipmapped::kNo);
    if (!characterization.isValid())
      return nullptr;
    auto surface = SkSurfaces::RenderTarget(
        device->context.get(), characterization, skgpu::Budgeted::kYes);
    if (!surface)
      return nullptr;
    auto result = std::make_unique<NativeBuffer>(std::move(characterization),
                                                 std::move(surface));
    result->context = device->context;
    result->gl = device->gl;
    result->device = device->device;
    result->surface->getCanvas()->clear(SK_ColorTRANSPARENT);
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
NativeAccess *scoop_graphics_gpu_access(SGContext *device) {
  try {
    auto result = std::make_unique<NativeAccess>();
    result->context = device->context;
    result->gl = device->gl;
    result->device = device->device;
    if (!device->context->flushAndSubmit(GrSyncCpu::kNo).fSuccess)
      return nullptr;
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
NativeAccess *scoop_graphics_gpu_write(SGTarget *target) {
  try {
    if (!target->prepareWrite())
      return nullptr;
    auto result = std::make_unique<NativeAccess>();
    result->context = target->context;
    result->gl = target->gl;
    result->device = target->device;
    result->surface = target->surface;
    // Flush retained pixels before native commands use the current allocation.
    result->texture = SkSurfaces::GetBackendTexture(
        target->surface.get(), SkSurfaces::BackendHandleAccess::kFlushWrite);
    if (!result->texture.isValid() || !target->context->submit(GrSyncCpu::kNo))
      return nullptr;
    result->completion = sg_gpu_completion(result->device, result->texture);
    if (!result->completion)
      return nullptr;
    return result.release();
  } catch (...) {
    return nullptr;
  }
}
SGDeviceInfo scoop_graphics_gpu_access_device(NativeAccess *access) {
  return access->device;
}
SGTextureInfo scoop_graphics_gpu_access_texture(NativeAccess *access) {
  return sg_gpu_texture(access->texture);
}
int32_t scoop_graphics_gpu_access_end(NativeAccess *access) {
  try {
    access->context->resetContext();
    access->ended = true;
    // Skia skips empty command buffers; its finished callbacks cannot fence
    // externally submitted work. Each backend uses its native queue instead.
    return !access->completion || access->completion.release()->submit();
  } catch (...) {
    return 0;
  }
}
SGImage *scoop_graphics_gpu_import(SGContext *device, SGTextureInfo texture,
                                   int32_t width, int32_t height,
                                   uint64_t format, int32_t layout,
                                   SGImageOwner owner, bool bottomLeft,
                                   bool linear, int32_t alpha) {
  try {
    auto imported = std::make_unique<ImportedTexture>(owner, texture.state);
    imported->texture =
        sg_gpu_import(device->device, texture, width, height, format);
    if (!imported->texture.isValid())
      return nullptr;
    auto backend = imported->texture;
    // BorrowTextureFrom consumes the release callback on both success and
    // failure.
    auto image = SkImages::BorrowTextureFrom(
        device->context.get(), backend,
        bottomLeft ? kBottomLeft_GrSurfaceOrigin : kTopLeft_GrSurfaceOrigin,
        color_type(layout, format),
        alpha == 0   ? kPremul_SkAlphaType
        : alpha == 1 ? kUnpremul_SkAlphaType
                     : kOpaque_SkAlphaType,
        color_space(linear), release_texture, imported.release());
    return image ? new SGImage(std::move(image), device->context, device->gl)
                 : nullptr;
  } catch (...) {
    return nullptr;
  }
}
}
