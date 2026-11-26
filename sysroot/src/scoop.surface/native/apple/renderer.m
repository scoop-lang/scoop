#import <TargetConditionals.h>
#if TARGET_OS_IOS
#import <UIKit/UIKit.h>
#import <objc/runtime.h>
typedef UIView SSUView;
static const char rendererKey;
#else
#import "../darwin/fps.h"
#import <AppKit/AppKit.h>
typedef NSView SSUView;
#endif
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalDisplayLink.h>
#import <QuartzCore/CATransaction.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef int32_t (*SSUCallback)(void *, int32_t);
typedef struct {
  void *pointer;
  int32_t status;
} SSUAcquired;

@interface SSUDevice : NSObject
@property(nonatomic, strong) id<MTLDevice> device;
@property(nonatomic, strong) id<MTLCommandQueue> queue;
@end
@implementation SSUDevice
@end

@interface SSUSurface : NSObject <CAMetalDisplayLinkDelegate>
@property(nonatomic, strong) SSUDevice *device;
@property(nonatomic, strong) SSUView *view;
#if TARGET_OS_IOS
@property(nonatomic) BOOL sceneActive;
#else
@property(nonatomic, strong) CALayer *previousLayer;
@property(nonatomic) BOOL previousWantsLayer;
#endif
@property(nonatomic, strong) CAMetalLayer *layer;
@property(nonatomic, strong) CAMetalDisplayLink *link;
@property(nonatomic, strong) id<CAMetalDrawable> pending;
@property(nonatomic, strong) id<MTLCommandBuffer> lastCommand;
#if !TARGET_OS_IOS
@property(nonatomic, strong) SSUMetalFpsProbe *fpsProbe;
#endif
@property(nonatomic) int32_t width;
@property(nonatomic) int32_t height;
@property(nonatomic) double scale;
@property(nonatomic) uint64_t generation;
@property(nonatomic) BOOL required;
@property(nonatomic) BOOL notified;
@property(nonatomic) BOOL closed;
@property(nonatomic) BOOL failed;
@property(nonatomic) SSUCallback callback;
@property(nonatomic) void *context;
- (void)updateDemand;
- (BOOL)restoreLayer;
- (BOOL)stopLink;
@end

@interface SSUFrame : NSObject
@property(nonatomic, weak) SSUSurface *surface;
@property(nonatomic, strong) id<CAMetalDrawable> drawable;
@property(nonatomic) uint64_t generation;
@end
@implementation SSUFrame
@end

@implementation SSUSurface
- (void)updateDemand {
  self.link.paused =
      self.closed || !self.required || self.width <= 0 || self.height <= 0;
#if TARGET_OS_IOS
  if (!self.sceneActive)
    self.link.paused = YES;
#endif
  if (self.link.paused) {
    self.pending = nil;
    self.notified = NO;
  }
}
- (void)metalDisplayLink:(CAMetalDisplayLink *)link
             needsUpdate:(CAMetalDisplayLinkUpdate *)update {
  if (self.closed || !self.required)
    return;
  self.pending = update.drawable;
  if (!self.notified) {
    self.notified = YES;
    if (!self.callback(self.context, 1))
      link.paused = YES;
  }
}
#if TARGET_OS_IOS
- (void)sceneActivationChanged:(NSNotification *)note {
  self.sceneActive = [note.name isEqualToString:UISceneDidActivateNotification];
  self.generation++;
  [self updateDemand];
}
- (void)sceneEnteredBackground:(NSNotification *)note {
  [self.lastCommand waitUntilScheduled];
}
#endif
- (BOOL)stopLink {
  self.closed = YES;
  self.required = NO;
  self.callback = NULL;
  self.context = NULL;
  @try {
#if TARGET_OS_IOS
    [NSNotificationCenter.defaultCenter removeObserver:self];
#endif
    [self.link invalidate];
    self.link.delegate = nil;
    self.link = nil;
    self.pending = nil;
    return YES;
  } @catch (NSException *exception) {
    return NO;
  }
}
- (BOOL)restoreLayer {
  @try {
#if TARGET_OS_IOS
    if (objc_getAssociatedObject(self.view, &rendererKey) == self) {
      objc_setAssociatedObject(self.view, &rendererKey, nil,
                               OBJC_ASSOCIATION_ASSIGN);
      self.layer.device = nil;
    }
#else
    if (self.view.layer == self.layer) {
      self.view.layer = self.previousLayer;
      self.view.wantsLayer = self.previousWantsLayer;
    }
    self.previousLayer = nil;
#endif
    self.view = nil;
    self.layer = nil;
    return YES;
  } @catch (NSException *exception) {
    return NO;
  }
}
@end

void *scoop_surface_metal_host_create(void) {
  @autoreleasepool {
    @try {
      SSUDevice *host = [SSUDevice new];
      host.device = MTLCreateSystemDefaultDevice();
      host.queue = [host.device newCommandQueue];
      return host.device && host.queue ? (__bridge_retained void *)host : NULL;
    } @catch (NSException *exception) {
      return NULL;
    }
  }
}
void *scoop_surface_metal_host_object(void *pointer, int32_t kind) {
  SSUDevice *host = (__bridge SSUDevice *)pointer;
  return kind == 0 ? (__bridge void *)host.device : (__bridge void *)host.queue;
}
void scoop_surface_metal_host_close(void *pointer) {
  SSUDevice *host = (__bridge_transfer SSUDevice *)pointer;
  (void)host;
}
void *scoop_surface_metal_surface_create(void *hostPointer, void *viewPointer,
                                         SSUCallback callback, void *context) {
  @autoreleasepool {
    SSUSurface *surface = nil;
    @try {
      SSUView *view = (__bridge SSUView *)viewPointer;
#if TARGET_OS_IOS
      UIWindowScene *scene = view.window.windowScene;
      if (!scene || ![view.layer isKindOfClass:CAMetalLayer.class] ||
          objc_getAssociatedObject(view, &rendererKey))
        return NULL;
#else
      if ([view.layer isKindOfClass:CAMetalLayer.class])
        return NULL;
#endif
      surface = [SSUSurface new];
      surface.device = (__bridge SSUDevice *)hostPointer;
      surface.view = view;
      surface.callback = callback;
      surface.context = context;
#if TARGET_OS_IOS
      surface.layer = (CAMetalLayer *)view.layer;
      objc_setAssociatedObject(view, &rendererKey, surface,
                               OBJC_ASSOCIATION_ASSIGN);
      surface.sceneActive =
          scene.activationState == UISceneActivationStateForegroundActive;
      NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
      for (NSNotificationName name in @[
             UISceneDidActivateNotification, UISceneWillDeactivateNotification
           ])
        [center addObserver:surface
                   selector:@selector(sceneActivationChanged:)
                       name:name
                     object:scene];
      [center addObserver:surface
                 selector:@selector(sceneEnteredBackground:)
                     name:UISceneDidEnterBackgroundNotification
                   object:scene];
#else
      surface.previousLayer = view.layer;
      surface.previousWantsLayer = view.wantsLayer;
      surface.fpsProbe =
          [SSUMetalFpsProbe probeForWindowNumber:view.window.windowNumber];
      surface.layer = [CAMetalLayer layer];
      surface.layer.frame = view.bounds;
      surface.layer.autoresizingMask =
          kCALayerWidthSizable | kCALayerHeightSizable;
      view.wantsLayer = YES;
      view.layer = surface.layer;
#endif
      surface.layer.device = surface.device.device;
      surface.layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
      surface.layer.framebufferOnly = NO;
      surface.layer.opaque = view.window.opaque;
      CGColorSpaceRef color = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
      surface.layer.colorspace = color;
      CGColorSpaceRelease(color);
      surface.link =
          [[CAMetalDisplayLink alloc] initWithMetalLayer:surface.layer];
      if (!surface.link) {
        [surface stopLink];
        [surface restoreLayer];
        return NULL;
      }
      surface.link.paused = YES;
      surface.link.delegate = surface;
      [surface.link addToRunLoop:NSRunLoop.mainRunLoop
                         forMode:NSRunLoopCommonModes];
      return (__bridge_retained void *)surface;
    } @catch (NSException *exception) {
      [surface stopLink];
      [surface restoreLayer];
      return NULL;
    }
  }
}
int32_t scoop_surface_metal_surface_configure(void *pointer, int32_t width,
                                              int32_t height, double scale) {
  @try {
    SSUSurface *surface = (__bridge SSUSurface *)pointer;
    if (surface.closed || surface.view.layer != surface.layer)
      return -1;
    if (surface.width != width || surface.height != height ||
        surface.scale != scale) {
      surface.generation += 1;
      surface.pending = nil;
      surface.notified = NO;
      surface.width = width;
      surface.height = height;
      surface.scale = scale;
      [CATransaction begin];
      @try {
        [CATransaction setDisableActions:YES];
#if !TARGET_OS_IOS
        surface.layer.frame = surface.view.bounds;
#endif
        surface.layer.contentsScale = scale;
        if (width > 0 && height > 0)
          surface.layer.drawableSize = CGSizeMake(width, height);
      } @finally {
        [CATransaction commit];
      }
    }
    [surface updateDemand];
    return 1;
  } @catch (NSException *exception) {
    return -1;
  }
}
int32_t scoop_surface_metal_surface_demand(void *pointer, bool required) {
  @try {
    SSUSurface *surface = (__bridge SSUSurface *)pointer;
    surface.required = required;
    [surface updateDemand];
    return 1;
  } @catch (NSException *exception) {
    return -1;
  }
}
SSUAcquired scoop_surface_metal_surface_acquire(void *pointer) {
  @autoreleasepool {
    @try {
      SSUSurface *surface = (__bridge SSUSurface *)pointer;
      if (surface.closed || surface.failed ||
          surface.view.layer != surface.layer)
        return (SSUAcquired){NULL, -1};
      id<CAMetalDrawable> drawable = surface.pending;
      surface.pending = nil;
      surface.notified = NO;
      if (!drawable || drawable.texture.width != (NSUInteger)surface.width ||
          drawable.texture.height != (NSUInteger)surface.height)
        return (SSUAcquired){NULL, 0};
      SSUFrame *frame = [SSUFrame new];
      frame.surface = surface;
      frame.drawable = drawable;
      frame.generation = surface.generation;
      return (SSUAcquired){(__bridge_retained void *)frame, 1};
    } @catch (NSException *exception) {
      return (SSUAcquired){NULL, -1};
    }
  }
}
void *scoop_surface_metal_frame_texture(void *pointer) {
  SSUFrame *frame = (__bridge SSUFrame *)pointer;
  return (__bridge void *)frame.drawable.texture;
}
int32_t scoop_surface_metal_frame_current(void *pointer) {
  @try {
    SSUFrame *frame = (__bridge SSUFrame *)pointer;
    SSUSurface *surface = frame.surface;
    if (!surface || surface.closed || frame.generation != surface.generation)
      return 0;
    if (surface.failed || surface.view.layer != surface.layer)
      return -1;
    // Native resizing can precede delivery of the queued geometry event.
#if TARGET_OS_IOS
    CGFloat scale = surface.view.traitCollection.displayScale;
    CGSize bounds = surface.view.bounds.size;
    return surface.sceneActive && surface.view.window &&
           scale == surface.scale &&
           ceil(bounds.width * scale) == surface.width &&
           ceil(bounds.height * scale) == surface.height;
#else
    NSRect pixels = [surface.view convertRectToBacking:surface.view.bounds];
    return ceil(pixels.size.width) == surface.width &&
           ceil(pixels.size.height) == surface.height &&
           surface.view.window.backingScaleFactor == surface.scale;
#endif
  } @catch (NSException *exception) {
    return -1;
  }
}
int32_t scoop_surface_metal_frame_present(void *pointer) {
  @autoreleasepool {
    @try {
      SSUFrame *frame = (__bridge SSUFrame *)pointer;
      SSUSurface *surface = frame.surface;
      if (!surface || surface.closed || frame.generation != surface.generation)
        return 0;
      if (surface.failed || surface.view.layer != surface.layer)
        return -1;
      id<MTLCommandBuffer> command = [surface.device.queue commandBuffer];
      if (!command)
        return -1;
#if !TARGET_OS_IOS
      [surface.fpsProbe watchDrawable:frame.drawable];
#endif
      // CAMetalDisplayLink schedules its drawables and rejects timed present.
      [command presentDrawable:frame.drawable];
      __weak SSUSurface *weakSurface = surface;
      [command addCompletedHandler:^(id<MTLCommandBuffer> completed) {
        frame.drawable = nil;
        if (completed.status == MTLCommandBufferStatusError) {
          CFRunLoopPerformBlock(CFRunLoopGetMain(), kCFRunLoopCommonModes, ^{
            SSUSurface *owner = weakSurface;
            if (owner && !owner.closed) {
              owner.failed = YES;
              owner.callback(owner.context, 2);
            }
          });
          CFRunLoopWakeUp(CFRunLoopGetMain());
        }
      }];
      surface.lastCommand = command;
      [command commit];
      return 1;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}
void scoop_surface_metal_frame_close(void *pointer) {
  SSUFrame *frame = (__bridge_transfer SSUFrame *)pointer;
  (void)frame;
}
int32_t scoop_surface_metal_surface_close(void *pointer) {
  SSUSurface *surface = (__bridge_transfer SSUSurface *)pointer;
  int32_t status = [surface stopLink] ? 1 : -1;
  @try {
#if TARGET_OS_IOS
    [surface.lastCommand waitUntilCompleted];
#else
    id<MTLCommandBuffer> fence = [surface.device.queue commandBuffer];
    if (fence) {
      [fence commit];
      [fence waitUntilCompleted];
      if (fence.status == MTLCommandBufferStatusError)
        status = -1;
    } else
      status = -1;
#endif
    if (surface.failed ||
        surface.lastCommand.status == MTLCommandBufferStatusError)
      status = -1;
  } @catch (NSException *exception) {
    status = -1;
  }
  if (![surface restoreLayer])
    status = -1;
#if !TARGET_OS_IOS
  [surface.fpsProbe finish];
#endif
  return status;
}
