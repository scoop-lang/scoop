#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#include <dispatch/dispatch.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>

void *scoop_graphics_test_gpu_delayed_write(void *queue, void *texture) {
  @autoreleasepool {
    id<MTLCommandQueue> nativeQueue = (__bridge id<MTLCommandQueue>)queue;
    id<MTLSharedEvent> event = [nativeQueue.device newSharedEvent];
    MTLCommandBufferDescriptor *options = [MTLCommandBufferDescriptor new];
    options.retainedReferences = NO;
    id<MTLCommandBuffer> command =
        [nativeQueue commandBufferWithDescriptor:options];
    [command encodeWaitForEvent:event value:1];
    MTLRenderPassDescriptor *pass =
        [MTLRenderPassDescriptor renderPassDescriptor];
    pass.colorAttachments[0].texture = (__bridge id<MTLTexture>)texture;
    pass.colorAttachments[0].loadAction = MTLLoadActionClear;
    pass.colorAttachments[0].storeAction = MTLStoreActionStore;
    pass.colorAttachments[0].clearColor = MTLClearColorMake(0, 0, 1, 1);
    id<MTLRenderCommandEncoder> encoder =
        [command renderCommandEncoderWithDescriptor:pass];
    [encoder endEncoding];
    [command commit];
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 50 * NSEC_PER_MSEC),
                   dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
                     event.signaledValue = 1;
                   });
    return (__bridge_retained void *)command;
  }
}
bool scoop_graphics_test_gpu_write_complete(void *pointer) {
  return ((__bridge id<MTLCommandBuffer>)pointer).status ==
         MTLCommandBufferStatusCompleted;
}
void scoop_graphics_test_gpu_write_close(void *pointer) {
  id<MTLCommandBuffer> command =
      (__bridge_transfer id<MTLCommandBuffer>)pointer;
  [command waitUntilCompleted];
}

@interface SGTestGpuFrame : NSObject {
@public
  atomic_int retains, releases, references;
}
@property(nonatomic, strong) id<MTLDevice> device;
@property(nonatomic, strong) id<MTLCommandQueue> queue;
@property(nonatomic, strong) id<MTLTexture> texture;
@end
@implementation SGTestGpuFrame
@end

uint64_t scoop_graphics_test_gpu_metal_format(void *texture) {
  return ((__bridge id<MTLTexture>)texture).pixelFormat;
}

void *scoop_graphics_test_gpu_create(void *first, void *second, void *third,
                                     int32_t width, int32_t height,
                                     uint64_t format) {
  (void)third;
  @autoreleasepool {
    SGTestGpuFrame *frame = [SGTestGpuFrame new];
    frame.device = (__bridge id<MTLDevice>)first;
    frame.queue = (__bridge id<MTLCommandQueue>)second;
    MTLTextureDescriptor *desc = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:(MTLPixelFormat)format
                                     width:(NSUInteger)width
                                    height:(NSUInteger)height
                                 mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead | MTLTextureUsageRenderTarget;
    desc.storageMode = MTLStorageModePrivate;
    frame.texture = [frame.device newTextureWithDescriptor:desc];
    atomic_init(&frame->retains, 0);
    atomic_init(&frame->releases, 0);
    atomic_init(&frame->references, 1);
    return frame.texture ? (__bridge_retained void *)frame : NULL;
  }
}
void *scoop_graphics_test_gpu_texture(void *pointer) {
  return (__bridge void *)((__bridge SGTestGpuFrame *)pointer).texture;
}
uint32_t scoop_graphics_test_gpu_name(void *pointer) {
  (void)pointer;
  return 0;
}
int32_t scoop_graphics_test_gpu_metric(void *pointer, int32_t kind) {
  SGTestGpuFrame *frame = (__bridge SGTestGpuFrame *)pointer;
  switch (kind) {
  case 0:
    return atomic_load(&frame->retains);
  case 1:
    return atomic_load(&frame->releases);
  case 2:
    return atomic_load(&frame->references);
  default:
    return 0;
  }
}
static void retain_frame(void *pointer) {
  SGTestGpuFrame *frame = (__bridge SGTestGpuFrame *)pointer;
  atomic_fetch_add(&frame->retains, 1);
  atomic_fetch_add(&frame->references, 1);
  CFRetain(pointer);
}
static void release_frame(void *pointer, uint32_t state) {
  (void)state;
  SGTestGpuFrame *frame = (__bridge SGTestGpuFrame *)pointer;
  atomic_fetch_add(&frame->releases, 1);
  atomic_fetch_sub(&frame->references, 1);
  CFRelease(pointer);
}
void (*scoop_graphics_test_gpu_retain(void))(void *) { return retain_frame; }
void (*scoop_graphics_test_gpu_release(void))(void *, uint32_t) {
  return release_frame;
}
bool scoop_graphics_test_gpu_fill(void *pointer, void *texture, uint32_t name,
                                  uint32_t state, double r, double g, double b,
                                  double a) {
  (void)name;
  (void)state;
  @autoreleasepool {
    SGTestGpuFrame *frame = (__bridge SGTestGpuFrame *)pointer;
    MTLRenderPassDescriptor *desc =
        [MTLRenderPassDescriptor renderPassDescriptor];
    desc.colorAttachments[0].texture = (__bridge id<MTLTexture>)texture;
    desc.colorAttachments[0].loadAction = MTLLoadActionClear;
    desc.colorAttachments[0].storeAction = MTLStoreActionStore;
    desc.colorAttachments[0].clearColor = MTLClearColorMake(r, g, b, a);
    id<MTLCommandBuffer> commands = [frame.queue commandBuffer];
    id<MTLRenderCommandEncoder> encoder =
        [commands renderCommandEncoderWithDescriptor:desc];
    if (!encoder)
      return false;
    [encoder endEncoding];
    [commands commit];
    return true;
  }
}
bool scoop_graphics_test_gpu_close(void *pointer) {
  @autoreleasepool {
    SGTestGpuFrame *frame = (__bridge SGTestGpuFrame *)pointer;
    id<MTLCommandBuffer> barrier = [frame.queue commandBuffer];
    [barrier commit];
    [barrier waitUntilCompleted];
    bool ok = barrier.status == MTLCommandBufferStatusCompleted;
    atomic_fetch_sub(&frame->references, 1);
    CFRelease(pointer);
    return ok;
  }
}
