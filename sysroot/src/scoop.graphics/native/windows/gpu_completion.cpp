#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include "../shared/gpu.h"
#include "include/gpu/ganesh/d3d/GrD3DBackendSurface.h"
#include "include/gpu/ganesh/d3d/GrD3DTypes.h"

namespace {
struct D3DCompletion final : SGCompletion {
  gr_cp<ID3D12CommandQueue> queue;
  gr_cp<ID3D12Fence> fence;
  GrD3DTextureResourceInfo texture;
  HANDLE event = nullptr;
  PTP_WAIT wait = nullptr;
  ~D3DCompletion() override {
    if (wait)
      CloseThreadpoolWait(wait);
    if (event)
      CloseHandle(event);
  }
  bool submit() override {
    std::unique_ptr<D3DCompletion> self(this);
    if (FAILED(queue->Signal(fence.get(), 1)))
      return false;
    // The callback may run before SetThreadpoolWait returns. Transfer ownership
    // first.
    auto *pending = self.release();
    SetThreadpoolWait(pending->wait, pending->event, nullptr);
    return true;
  }
};
void CALLBACK complete(PTP_CALLBACK_INSTANCE, void *pointer, PTP_WAIT,
                       TP_WAIT_RESULT) {
  // Only native resource/allocation references and Win32 handles are released
  // here.
  delete static_cast<D3DCompletion *>(pointer);
}
} // namespace

std::unique_ptr<SGCompletion>
sg_gpu_completion(const SGDeviceInfo &info, const GrBackendTexture &texture) {
  auto *device = static_cast<ID3D12Device *>(const_cast<void *>(info.second));
  auto result = std::make_unique<D3DCompletion>();
  result->queue.retain(
      static_cast<ID3D12CommandQueue *>(const_cast<void *>(info.third)));
  result->texture = GrBackendTextures::GetD3DTextureResourceInfo(texture);
  if (FAILED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE,
                                 IID_PPV_ARGS(&result->fence))))
    return nullptr;
  result->event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
  result->wait = CreateThreadpoolWait(complete, result.get(), nullptr);
  if (!result->event || !result->wait ||
      FAILED(result->fence->SetEventOnCompletion(1, result->event)))
    return nullptr;
  return result;
}
bool sg_gpu_drain_native(const SGDeviceInfo &info) {
  auto *device = static_cast<ID3D12Device *>(const_cast<void *>(info.second));
  auto *queue =
      static_cast<ID3D12CommandQueue *>(const_cast<void *>(info.third));
  gr_cp<ID3D12Fence> fence;
  HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
  bool ok = event &&
            SUCCEEDED(device->CreateFence(0, D3D12_FENCE_FLAG_NONE,
                                          IID_PPV_ARGS(&fence))) &&
            SUCCEEDED(queue->Signal(fence.get(), 1)) &&
            SUCCEEDED(fence->SetEventOnCompletion(1, event)) &&
            WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0;
  if (event)
    CloseHandle(event);
  return ok;
}
