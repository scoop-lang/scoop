#import "window.h"
#import <CoreGraphics/CoreGraphics.h>
#include <math.h>

typedef struct {
  int64_t key, primary, refresh_millihertz;
  double x, y, width, height, pixel_width, pixel_height, scale;
  const uint8_t *name;
  int64_t name_length;
  double work_x, work_y, work_width, work_height;
} SWMonitorData;

@interface SWMonitorRecord : NSObject
@property(nonatomic) SWMonitorData data;
@property(nonatomic, strong) NSData *name;
@end
@implementation SWMonitorRecord
@end

void *scoop_window_monitors(void) {
  @autoreleasepool {
    NSArray<NSScreen *> *screens = NSScreen.screens;
    NSMutableArray<SWMonitorRecord *> *result = [NSMutableArray new];
    double top = NSMaxY(screens.firstObject.frame);
    for (NSScreen *screen in screens) {
      SWMonitorRecord *record = [SWMonitorRecord new];
      SWMonitorData data = {0};
      CGDirectDisplayID display =
          [screen.deviceDescription[@"NSScreenNumber"] unsignedIntValue];
      data.key = display;
      data.primary = screen == screens.firstObject;
      NSRect frame = screen.frame;
      NSRect pixels = [screen convertRectToBacking:frame];
      data.x = frame.origin.x;
      data.y = top - NSMaxY(frame);
      data.width = frame.size.width;
      data.height = frame.size.height;
      NSRect work = screen.visibleFrame;
      data.work_x = work.origin.x;
      data.work_y = top - NSMaxY(work);
      data.work_width = work.size.width;
      data.work_height = work.size.height;
      data.pixel_width = ceil(pixels.size.width);
      data.pixel_height = ceil(pixels.size.height);
      data.scale = screen.backingScaleFactor;
      CGDisplayModeRef mode = CGDisplayCopyDisplayMode(display);
      if (mode) {
        double millihertz = CGDisplayModeGetRefreshRate(mode) * 1000.0;
        if (isfinite(millihertz) && millihertz > 0 &&
            millihertz < (double)INT64_MAX)
          data.refresh_millihertz = llround(millihertz);
        CGDisplayModeRelease(mode);
      }
      record.data = data;
      record.name =
          [screen.localizedName dataUsingEncoding:NSUTF8StringEncoding];
      [result addObject:record];
    }
    return (__bridge_retained void *)[result copy];
  }
}

int64_t scoop_window_monitors_count(void *pointer) {
  return (int64_t)((__bridge NSArray *)pointer).count;
}

SWMonitorData scoop_window_monitor_data(void *pointer, int64_t index) {
  SWMonitorRecord *record = ((__bridge NSArray *)pointer)[(NSUInteger)index];
  SWMonitorData data = record.data;
  data.name = record.name.bytes;
  data.name_length = (int64_t)record.name.length;
  return data;
}

void scoop_window_monitors_release(void *pointer) {
  (void)CFBridgingRelease(pointer);
}

int64_t scoop_window_current_monitor(void *pointer) {
  NSScreen *screen = ((__bridge SWWindow *)pointer).window.screen;
  return [screen.deviceDescription[@"NSScreenNumber"] unsignedIntValue];
}

typedef struct {
  int64_t error;
  double x, y, width, height;
} SWDesktopRect;

int32_t scoop_window_desktop_capabilities(void) { return 1 | 2 | 8; }
SWDesktopRect scoop_window_system_taskbar_bounds(void) {
  return (SWDesktopRect){.error = -1};
}
double scoop_window_system_menu_bar_height(void) {
  return NSStatusBar.systemStatusBar.thickness;
}
