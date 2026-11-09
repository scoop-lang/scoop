#import <Metal/Metal.h>
#import <objc/runtime.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
  int64_t buffers, completed, passes, pixels, large_passes;
  double gpu_ms;
} HoverGpuStats;

static _Atomic(bool) enabled;
static _Atomic(int64_t) buffers, completed, passes, pixels, large_passes,
    gpu_ns;
static IMP commit_impl, encoder_impl;

static void observe_commit(id<MTLCommandBuffer> command, SEL selector) {
  if (atomic_load(&enabled)) {
    atomic_fetch_add(&buffers, 1);
    [command addCompletedHandler:^(id<MTLCommandBuffer> done) {
      atomic_fetch_add(&gpu_ns,
                       (int64_t)((done.GPUEndTime - done.GPUStartTime) * 1e9));
      atomic_fetch_add(&completed, 1);
    }];
  }
  ((void (*)(id, SEL))commit_impl)(command, selector);
}
static id<MTLRenderCommandEncoder>
observe_encoder(id command, SEL selector, MTLRenderPassDescriptor *descriptor) {
  if (atomic_load(&enabled)) {
    id<MTLTexture> texture = descriptor.colorAttachments[0].texture;
    int64_t area = (int64_t)texture.width * (int64_t)texture.height;
    int64_t index = atomic_fetch_add(&passes, 1);
    if (index < 12 && getenv("SCOOP_UI_GPU_TRACE"))
      fprintf(stderr,
              "pass=%lld texture=%lux%lu samples=%lu format=%lu label=%s\n",
              (long long)index, (unsigned long)texture.width,
              (unsigned long)texture.height, (unsigned long)texture.sampleCount,
              (unsigned long)texture.pixelFormat,
              texture.label ? texture.label.UTF8String : "");
    atomic_fetch_add(&pixels, area);
    if (area > 500000)
      atomic_fetch_add(&large_passes, 1);
  }
  return ((id (*)(id, SEL, id))encoder_impl)(command, selector, descriptor);
}
int32_t hover_gpu_install(void) {
  id<MTLDevice> device = MTLCreateSystemDefaultDevice();
  id<MTLCommandQueue> queue = [device newCommandQueue];
  id<MTLCommandBuffer> command = [queue commandBuffer];
  Class type = object_getClass(command);
  Method commit = class_getInstanceMethod(type, @selector(commit));
  Method encoder = class_getInstanceMethod(
      type, @selector(renderCommandEncoderWithDescriptor:));
  if (!commit || !encoder)
    return 0;
  commit_impl = method_setImplementation(commit, (IMP)observe_commit);
  encoder_impl = method_setImplementation(encoder, (IMP)observe_encoder);
  fprintf(stderr, "GPU observation: %s, %s\n", device.name.UTF8String,
          class_getName(type));
  return 1;
}
void hover_gpu_begin(void) {
  atomic_store(&buffers, 0);
  atomic_store(&completed, 0);
  atomic_store(&passes, 0);
  atomic_store(&pixels, 0);
  atomic_store(&large_passes, 0);
  atomic_store(&gpu_ns, 0);
  atomic_store(&enabled, true);
}
HoverGpuStats hover_gpu_end(void) {
  atomic_store(&enabled, false);
  return (HoverGpuStats){
      atomic_load(&buffers),      atomic_load(&completed),
      atomic_load(&passes),       atomic_load(&pixels),
      atomic_load(&large_passes), atomic_load(&gpu_ns) / 1e6};
}
void hover_gpu_report(void) {
  HoverGpuStats stats = hover_gpu_end();
  printf("{\"buffers\":%lld,\"completed\":%lld,\"passes\":%lld,\"target_"
         "pixels\":%lld,\"large_passes\":%lld,\"gpu_ms\":%.4f}\n",
         (long long)stats.buffers, (long long)stats.completed,
         (long long)stats.passes, (long long)stats.pixels,
         (long long)stats.large_passes, stats.gpu_ms);
  fflush(stdout);
}
