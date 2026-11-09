#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <atomic>
#include <cstdint>
#include <d3d12.h>
#include <memory>
#include <vector>
#include <windows.h>
#include <wrl/client.h>
using Microsoft::WRL::ComPtr;

struct Submission {
  ComPtr<ID3D12CommandAllocator> allocator;
  ComPtr<ID3D12GraphicsCommandList> commands;
  ComPtr<ID3D12DescriptorHeap> descriptors;
  ComPtr<ID3D12Resource> texture;
};
struct GpuFrame {
  std::atomic<int32_t> retains{0}, releases{0}, references{1};
  std::atomic<uint32_t> state{0};
  ComPtr<ID3D12Device> device;
  ComPtr<ID3D12CommandQueue> queue;
  ComPtr<ID3D12Resource> texture;
  std::vector<Submission> submissions;
};
static void drop(GpuFrame *frame) {
  if (--frame->references == 0)
    delete frame;
}
static void retain_frame(void *pointer) {
  auto *frame = static_cast<GpuFrame *>(pointer);
  ++frame->retains;
  ++frame->references;
}
static void release_frame(void *pointer, uint32_t state) {
  auto *frame = static_cast<GpuFrame *>(pointer);
  frame->state = state;
  ++frame->releases;
  drop(frame);
}
extern "C" {
GpuFrame *scoop_graphics_test_gpu_create(void *, ID3D12Device *device,
                                         ID3D12CommandQueue *queue,
                                         int32_t width, int32_t height,
                                         uint64_t format) {
  try {
    auto frame = std::make_unique<GpuFrame>();
    frame->device = device;
    frame->queue = queue;
    D3D12_HEAP_PROPERTIES heap = {};
    heap.Type = D3D12_HEAP_TYPE_DEFAULT;
    D3D12_RESOURCE_DESC desc = {};
    desc.Dimension = D3D12_RESOURCE_DIMENSION_TEXTURE2D;
    desc.Width = width;
    desc.Height = height;
    desc.DepthOrArraySize = 1;
    desc.MipLevels = 1;
    desc.Format = static_cast<DXGI_FORMAT>(format);
    desc.SampleDesc.Count = 1;
    desc.Flags = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET;
    if (FAILED(device->CreateCommittedResource(
            &heap, D3D12_HEAP_FLAG_NONE, &desc, D3D12_RESOURCE_STATE_COMMON,
            nullptr, IID_PPV_ARGS(&frame->texture))))
      return nullptr;
    return frame.release();
  } catch (...) {
    return nullptr;
  }
}
void *scoop_graphics_test_gpu_texture(GpuFrame *frame) {
  return frame->texture.Get();
}
uint32_t scoop_graphics_test_gpu_name(GpuFrame *) { return 0; }
int32_t scoop_graphics_test_gpu_metric(GpuFrame *frame, int32_t kind) {
  switch (kind) {
  case 0:
    return frame->retains;
  case 1:
    return frame->releases;
  case 2:
    return frame->references;
  default:
    return static_cast<int32_t>(frame->state.load());
  }
}
void (*scoop_graphics_test_gpu_retain())(void *) { return retain_frame; }
void (*scoop_graphics_test_gpu_release())(void *, uint32_t) {
  return release_frame;
}
bool scoop_graphics_test_gpu_fill(GpuFrame *frame, ID3D12Resource *texture,
                                  uint32_t, uint32_t state, double r, double g,
                                  double b, double a) {
  try {
    Submission job;
    job.texture = texture;
    if (FAILED(frame->device->CreateCommandAllocator(
            D3D12_COMMAND_LIST_TYPE_DIRECT, IID_PPV_ARGS(&job.allocator))) ||
        FAILED(frame->device->CreateCommandList(
            0, D3D12_COMMAND_LIST_TYPE_DIRECT, job.allocator.Get(), nullptr,
            IID_PPV_ARGS(&job.commands))))
      return false;
    D3D12_DESCRIPTOR_HEAP_DESC heap = {};
    heap.Type = D3D12_DESCRIPTOR_HEAP_TYPE_RTV;
    heap.NumDescriptors = 1;
    if (FAILED(frame->device->CreateDescriptorHeap(
            &heap, IID_PPV_ARGS(&job.descriptors))))
      return false;
    auto handle = job.descriptors->GetCPUDescriptorHandleForHeapStart();
    frame->device->CreateRenderTargetView(texture, nullptr, handle);
    D3D12_RESOURCE_BARRIER barrier = {};
    barrier.Type = D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
    barrier.Transition.pResource = texture;
    barrier.Transition.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES;
    barrier.Transition.StateBefore = static_cast<D3D12_RESOURCE_STATES>(state);
    barrier.Transition.StateAfter = D3D12_RESOURCE_STATE_RENDER_TARGET;
    if (barrier.Transition.StateBefore != barrier.Transition.StateAfter)
      job.commands->ResourceBarrier(1, &barrier);
    const float color[] = {static_cast<float>(r), static_cast<float>(g),
                           static_cast<float>(b), static_cast<float>(a)};
    job.commands->ClearRenderTargetView(handle, color, 0, nullptr);
    std::swap(barrier.Transition.StateBefore, barrier.Transition.StateAfter);
    if (barrier.Transition.StateBefore != barrier.Transition.StateAfter)
      job.commands->ResourceBarrier(1, &barrier);
    if (FAILED(job.commands->Close()))
      return false;
    ID3D12CommandList *commands = job.commands.Get();
    frame->submissions.push_back(std::move(job));
    frame->queue->ExecuteCommandLists(1, &commands);
    return true;
  } catch (...) {
    return false;
  }
}
bool scoop_graphics_test_gpu_close(GpuFrame *frame) {
  ComPtr<ID3D12Fence> fence;
  HANDLE event = CreateEventW(nullptr, FALSE, FALSE, nullptr);
  bool ok = event &&
            SUCCEEDED(frame->device->CreateFence(0, D3D12_FENCE_FLAG_NONE,
                                                 IID_PPV_ARGS(&fence))) &&
            SUCCEEDED(frame->queue->Signal(fence.Get(), 1)) &&
            SUCCEEDED(fence->SetEventOnCompletion(1, event)) &&
            WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0;
  if (event)
    CloseHandle(event);
  frame->submissions.clear();
  drop(frame);
  return ok;
}
}
