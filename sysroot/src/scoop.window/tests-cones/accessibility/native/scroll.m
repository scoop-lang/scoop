#import <AppKit/AppKit.h>
#include <math.h>
#include <stdint.h>

#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      return __LINE__;                                                         \
  } while (0)

@interface SWScrollProbe : NSObject
@property(nonatomic, strong) id group, bar;
@property(nonatomic) BOOL closed;
@end
@implementation SWScrollProbe
@end

static id find(id node, NSString *identifier) {
  if ([[node accessibilityIdentifier] isEqualToString:identifier])
    return node;
  for (id child in [node accessibilityChildren]) {
    id found = find(child, identifier);
    if (found)
      return found;
  }
  return nil;
}

void *scoop_ax_scroll_open(void *pointer) {
  NSView *view = (__bridge NSView *)pointer;
  SWScrollProbe *probe = [SWScrollProbe new];
  probe.group = find(view, @"scroll-container");
  probe.bar = find(view, @"scroll-axis");
  return (__bridge_retained void *)probe;
}

static BOOL value(id bar, double expected) {
  id result = [bar accessibilityValue];
  return [result isKindOfClass:NSNumber.class] &&
         fabs([result doubleValue] - expected) < 1e-9;
}

static int32_t operate(SWScrollProbe *probe, int32_t phase) {
  id bar = probe.bar, group = probe.group;
  CHECK(bar && group);
  switch (phase) {
  case 0:
    CHECK(
        [[bar accessibilityRole] isEqualToString:NSAccessibilityScrollBarRole]);
    CHECK([bar accessibilityParent] == group);
    CHECK([group accessibilityVerticalScrollBar] == bar);
    CHECK(![group accessibilityHorizontalScrollBar]);
    CHECK([bar accessibilityOrientation] == NSAccessibilityOrientationVertical);
    CHECK([[bar accessibilityMinValue] doubleValue] == 0.0);
    CHECK([[bar accessibilityMaxValue] doubleValue] == 1.0);
    CHECK(value(bar, 0.2));
    CHECK([bar accessibilityPerformIncrement]);
    CHECK(value(bar, 0.3));
    CHECK([bar isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    break;
  case 1:
    [bar setAccessibilityValue:@0.75];
    CHECK(value(bar, 0.75));
    break;
  case 2:
    [bar setAccessibilityValue:@1.1];
    [bar setAccessibilityValue:@(-0.1)];
    [bar setAccessibilityValue:@(NAN)];
    [bar setAccessibilityValue:@"0.5"];
    CHECK(value(bar, 0.75));
    break;
  case 3:
  case 4:
    CHECK(![bar
        isAccessibilitySelectorAllowed:@selector(setAccessibilityValue:)]);
    CHECK(![bar
        isAccessibilitySelectorAllowed:@selector(
                                           accessibilityPerformDecrement)]);
    CHECK(![bar accessibilityPerformDecrement]);
    [bar setAccessibilityValue:@0.5];
    CHECK(value(bar, 0.75));
    break;
  case 5:
    CHECK(![bar isAccessibilityElement]);
    CHECK(![group accessibilityVerticalScrollBar]);
    CHECK(![bar accessibilityPerformIncrement]);
    break;
  case 6:
    CHECK(![bar accessibilityValue]);
    CHECK(![bar accessibilityMinValue]);
    CHECK(![bar
        isAccessibilitySelectorAllowed:@selector(accessibilityOrientation)]);
    CHECK(![bar accessibilityPerformIncrement]);
    CHECK(![group accessibilityVerticalScrollBar]);
    break;
  case 7:
    CHECK(value(bar, 0));
    CHECK([[bar accessibilityMaxValue] doubleValue] == 0.0);
    CHECK([bar accessibilityOrientation] ==
          NSAccessibilityOrientationHorizontal);
    CHECK([group accessibilityHorizontalScrollBar] == bar);
    CHECK(![group accessibilityVerticalScrollBar]);
    CHECK(![bar accessibilityPerformIncrement]);
    break;
  case 8:
    CHECK(![bar isAccessibilityElement]);
    CHECK(![bar accessibilityRole]);
    CHECK(![bar accessibilityValue]);
    CHECK(
        ![bar isAccessibilitySelectorAllowed:@selector(accessibilityMaxValue)]);
    CHECK(![bar accessibilityPerformIncrement]);
    break;
  default:
    return -1;
  }
  return 0;
}

void scoop_ax_scroll_post(void *pointer, int32_t phase,
                          int32_t (*callback)(void *, int32_t, int32_t),
                          void *context) {
  SWScrollProbe *probe = (__bridge SWScrollProbe *)pointer;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (probe.closed)
      return;
    int32_t result;
    @try {
      result = operate(probe, phase);
    } @catch (NSException *error) {
      result = -2;
    }
    callback(context, -phase - 1, result);
  });
}

void scoop_ax_scroll_close(void *pointer) {
  SWScrollProbe *probe = CFBridgingRelease(pointer);
  probe.closed = YES;
}
