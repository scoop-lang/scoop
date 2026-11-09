#import <Metal/Metal.h>
#include <stdbool.h>
#include <stdint.h>

void *scoop_graphics_metal_completion(const void *queue, const void *pointer) {
  @autoreleasepool {
    id<MTLCommandBuffer> command =
        [(__bridge id<MTLCommandQueue>)queue commandBuffer];
    id<MTLTexture> texture = (__bridge id<MTLTexture>)pointer;
    [command addCompletedHandler:^(id<MTLCommandBuffer> done) {
      (void)done;
      (void)texture.width;
    }];
    return (__bridge_retained void *)command;
  }
}
bool scoop_graphics_metal_completion_submit(void *pointer) {
  id<MTLCommandBuffer> command = (__bridge id<MTLCommandBuffer>)pointer;
  [command commit];
  return command.status != MTLCommandBufferStatusError;
}
void scoop_graphics_metal_completion_drop(void *pointer) {
  if (pointer)
    CFRelease(pointer);
}
bool scoop_graphics_metal_queue_drain(const void *queue) {
  @autoreleasepool {
    id<MTLCommandBuffer> command =
        [(__bridge id<MTLCommandQueue>)queue commandBuffer];
    [command commit];
    [command waitUntilCompleted];
    return command && command.status == MTLCommandBufferStatusCompleted;
  }
}

// Ganesh aborts for formats outside its table, including valid Metal-only
// formats.
bool scoop_graphics_metal_format_supported(uint64_t format) {
  switch ((MTLPixelFormat)format) {
  case MTLPixelFormatRGBA8Unorm:
  case MTLPixelFormatR8Unorm:
  case MTLPixelFormatA8Unorm:
  case MTLPixelFormatBGRA8Unorm:
  case MTLPixelFormatB5G6R5Unorm:
  case MTLPixelFormatRGBA16Float:
  case MTLPixelFormatR16Float:
  case MTLPixelFormatRG8Unorm:
  case MTLPixelFormatRGB10A2Unorm:
  case MTLPixelFormatBGR10A2Unorm:
  case MTLPixelFormatABGR4Unorm:
  case MTLPixelFormatRGBA8Unorm_sRGB:
  case MTLPixelFormatR16Unorm:
  case MTLPixelFormatRG16Unorm:
  case MTLPixelFormatETC2_RGB8:
  case MTLPixelFormatBC1_RGBA:
  case MTLPixelFormatRGBA16Unorm:
  case MTLPixelFormatRG16Float:
    return true;
  default:
    return false;
  }
}

bool scoop_graphics_metal_texture_matches(const void *device,
                                          const void *pointer, int32_t width,
                                          int32_t height, uint64_t format) {
  id<MTLTexture> texture = (__bridge id<MTLTexture>)pointer;
  return scoop_graphics_metal_format_supported(format) && texture &&
         texture.device == (__bridge id<MTLDevice>)device &&
         (texture.usage == MTLTextureUsageUnknown ||
          (texture.usage & MTLTextureUsageShaderRead)) &&
         texture.textureType == MTLTextureType2D && texture.sampleCount == 1 &&
         texture.width == (NSUInteger)width &&
         texture.height == (NSUInteger)height &&
         texture.pixelFormat == (MTLPixelFormat)format;
}
