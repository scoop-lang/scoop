#import <AppKit/AppKit.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMediaTiming.h>
#import <QuartzCore/CAMetalDisplayLink.h>
#import <QuartzCore/CATransaction.h>
#include <math.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

@interface SSUMetalFpsProbe : NSObject
- (instancetype)initWithWindowNumber:(NSInteger)windowNumber;
- (void)watchDrawable:(id<MTLDrawable>)drawable;
- (void)finish;
@end

@implementation SSUMetalFpsProbe {
  NSInteger _windowNumber;
  CFTimeInterval _lastPresentation, _sampleStart;
  uint64_t _presented, _withoutTime, _sampleFrames;
  BOOL _closed;
}
- (instancetype)initWithWindowNumber:(NSInteger)windowNumber {
  self = [super init];
  if (self)
    _windowNumber = windowNumber;
  return self;
}
- (void)recordPresentedTime:(CFTimeInterval)time {
  CFTimeInterval start, end;
  uint64_t frames;
  @synchronized(self) {
    if (_closed)
      return;
    if (!isfinite(time) || time <= 0) {
      ++_withoutTime;
      return;
    }
    ++_presented;
    _lastPresentation = fmax(_lastPresentation, time);
    if (_sampleStart == 0)
      _sampleStart = time;
    // A late callback for a published interval still counts in the total.
    if (time <= _sampleStart)
      return;
    ++_sampleFrames;
    if (_lastPresentation - _sampleStart < 1.0)
      return;
    start = _sampleStart;
    end = _lastPresentation;
    frames = _sampleFrames;
    _sampleStart = end;
    _sampleFrames = 0;
  }
  fprintf(stderr,
          "scoop-surface-fps window=%ld start=%.6f end=%.6f frames=%llu "
          "seconds=%.6f fps=%.2f\n",
          (long)_windowNumber, start, end, (unsigned long long)frames,
          end - start, (double)frames / (end - start));
}
- (void)watchDrawable:(id<MTLDrawable>)drawable {
  [drawable addPresentedHandler:^(id<MTLDrawable> presented) {
    [self recordPresentedTime:presented.presentedTime];
  }];
}
- (void)finish {
  CFTimeInterval last;
  uint64_t presented, withoutTime;
  @synchronized(self) {
    _closed = YES;
    last = _lastPresentation;
    presented = _presented;
    withoutTime = _withoutTime;
  }
  fprintf(stderr,
          "scoop-surface-fps window=%ld closed_at=%.6f last_presented=%.6f "
          "presented_total=%llu without_time_total=%llu\n",
          (long)_windowNumber, CACurrentMediaTime(), last,
          (unsigned long long)presented, (unsigned long long)withoutTime);
}
@end

@interface SSUSurface : NSObject <CAMetalDisplayLinkDelegate>
@property(nonatomic, strong) SSUDevice *device;
@property(nonatomic, strong) NSView *view;
@property(nonatomic, strong) CALayer *previousLayer;
@property(nonatomic) BOOL previousWantsLayer;
@property(nonatomic, strong) CAMetalLayer *layer;
@property(nonatomic, strong) CAMetalDisplayLink *link;
@property(nonatomic, strong) id<CAMetalDrawable> pending;
@property(nonatomic, strong) id<MTLCommandBuffer> lastCommand;
@property(nonatomic, strong) SSUMetalFpsProbe *fpsProbe;
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
- (BOOL)stopLink {
  self.closed = YES;
  self.required = NO;
  self.callback = NULL;
  self.context = NULL;
  @try {
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
    if (self.view.layer == self.layer) {
      self.view.layer = self.previousLayer;
      self.view.wantsLayer = self.previousWantsLayer;
    }
    self.view = nil;
    self.previousLayer = nil;
    self.layer = nil;
    return YES;
  } @catch (NSException *exception) {
    return NO;
  }
}
@end

static atomic_bool fpsReportingEnabled = false;

void scoop_surface_metal_configure_fps_reporting_from_environment(void) {
  const char *fps = getenv("SCOOP_SURFACE_FPS");
  atomic_store_explicit(&fpsReportingEnabled, fps && strcmp(fps, "1") == 0,
                        memory_order_relaxed);
}

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
      NSView *view = (__bridge NSView *)viewPointer;
      if ([view.layer isKindOfClass:CAMetalLayer.class])
        return NULL;
      surface = [SSUSurface new];
      surface.device = (__bridge SSUDevice *)hostPointer;
      surface.view = view;
      surface.previousLayer = view.layer;
      surface.previousWantsLayer = view.wantsLayer;
      surface.callback = callback;
      surface.context = context;
      if (atomic_load_explicit(&fpsReportingEnabled, memory_order_relaxed))
        surface.fpsProbe = [[SSUMetalFpsProbe alloc]
            initWithWindowNumber:view.window.windowNumber];
      surface.layer = [CAMetalLayer layer];
      surface.layer.device = surface.device.device;
      surface.layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
      surface.layer.framebufferOnly = NO;
      surface.layer.opaque = view.window.opaque;
      surface.layer.frame = view.bounds;
      surface.layer.autoresizingMask =
          kCALayerWidthSizable | kCALayerHeightSizable;
      CGColorSpaceRef color = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
      surface.layer.colorspace = color;
      CGColorSpaceRelease(color);
      view.wantsLayer = YES;
      view.layer = surface.layer;
      surface.link =
          [[CAMetalDisplayLink alloc] initWithMetalLayer:surface.layer];
      if (!surface.link) {
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
        surface.layer.frame = surface.view.bounds;
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
    NSRect pixels = [surface.view convertRectToBacking:surface.view.bounds];
    return ceil(pixels.size.width) == surface.width &&
           ceil(pixels.size.height) == surface.height &&
           surface.view.window.backingScaleFactor == surface.scale;
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
      [surface.fpsProbe watchDrawable:frame.drawable];
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
    id<MTLCommandBuffer> fence = [surface.device.queue commandBuffer];
    if (fence) {
      [fence commit];
      [fence waitUntilCompleted];
      if (fence.status == MTLCommandBufferStatusError)
        status = -1;
    } else
      status = -1;
    if (surface.failed ||
        surface.lastCommand.status == MTLCommandBufferStatusError)
      status = -1;
  } @catch (NSException *exception) {
    status = -1;
  }
  if (![surface restoreLayer])
    status = -1;
  [surface.fpsProbe finish];
  return status;
}
