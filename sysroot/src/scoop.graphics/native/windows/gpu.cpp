#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include "../shared/gpu.h"
#include "include/gpu/ganesh/d3d/GrD3DBackendSurface.h"
#include "include/gpu/ganesh/d3d/GrD3DTypes.h"

GrBackendFormat sg_gpu_format(uint64_t format) {
  return GrBackendFormats::MakeD3D(static_cast<DXGI_FORMAT>(format));
}
bool sg_gpu_srgb(uint64_t format) {
  return format == DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;
}
GrBackendTexture sg_gpu_import(const SGDeviceInfo &device,
                               SGTextureInfo texture, int width, int height,
                               uint64_t format) {
  auto *resource =
      static_cast<ID3D12Resource *>(const_cast<void *>(texture.pointer));
  if (!resource)
    return {};
  const auto desc = resource->GetDesc();
  gr_cp<ID3D12Device> owner;
  if (FAILED(resource->GetDevice(IID_PPV_ARGS(&owner))) ||
      owner.get() != device.second ||
      desc.Dimension != D3D12_RESOURCE_DIMENSION_TEXTURE2D ||
      desc.SampleDesc.Count != 1 || desc.DepthOrArraySize != 1 ||
      desc.Width != static_cast<UINT64>(width) ||
      desc.Height != static_cast<UINT>(height) ||
      desc.Format != static_cast<DXGI_FORMAT>(format) ||
      (desc.Flags & D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE))
    return {};
  GrD3DTextureResourceInfo info;
  info.fResource.retain(resource);
  info.fResourceState = static_cast<D3D12_RESOURCE_STATES>(texture.state);
  info.fFormat = desc.Format;
  info.fLevelCount = 1;
  return GrBackendTextures::MakeD3D(width, height, info);
}
SGTextureInfo sg_gpu_texture(const GrBackendTexture &texture) {
  const auto info = GrBackendTextures::GetD3DTextureResourceInfo(texture);
  return {info.fResource.get(), 0, static_cast<uint32_t>(info.fResourceState)};
}
