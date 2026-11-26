#import "fps.h"
#import <QuartzCore/CAMediaTiming.h>
#include <math.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static atomic_bool fpsReportingEnabled = false;

void scoop_surface_metal_configure_fps_reporting_from_environment(void) {
  const char *fps = getenv("SCOOP_SURFACE_FPS");
  atomic_store_explicit(&fpsReportingEnabled, fps && strcmp(fps, "1") == 0,
                        memory_order_relaxed);
}

@implementation SSUMetalFpsProbe {
  NSInteger _windowNumber;
  CFTimeInterval _lastPresentation, _sampleStart;
  uint64_t _presented, _withoutTime, _sampleFrames;
  BOOL _closed;
}
+ (instancetype)probeForWindowNumber:(NSInteger)windowNumber {
  if (!atomic_load_explicit(&fpsReportingEnabled, memory_order_relaxed))
    return nil;
  return [[self alloc] initWithWindowNumber:windowNumber];
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
