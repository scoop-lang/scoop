#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

bool scoop_surface_test_scroll(void *pointer, double x, double y,
                               int32_t horizontal, int32_t vertical,
                               bool pixels, int32_t phase, int32_t momentum) {
  @autoreleasepool {
    @try {
      NSView *view = (__bridge NSView *)pointer;
      CGEventRef native = CGEventCreateScrollWheelEvent2(
          NULL, pixels ? kCGScrollEventUnitPixel : kCGScrollEventUnitLine, 2,
          vertical, horizontal, 0);
      if (!native)
        return false;
      // CG-created NSEvents have no NSWindow. For direct view delivery, encode
      // the intended locationInWindow instead of a desktop screen location.
      NSPoint location = [view convertPoint:NSMakePoint(x, y) toView:nil];
      CGFloat top = NSMaxY(NSScreen.screens.firstObject.frame);
      CGEventSetLocation(native, CGPointMake(location.x, top - location.y));
      CGEventSetIntegerValueField(native, kCGScrollWheelEventScrollPhase,
                                  phase);
      CGEventSetIntegerValueField(native, kCGScrollWheelEventMomentumPhase,
                                  momentum);
      NSEvent *event = [NSEvent eventWithCGEvent:native];
      CFRelease(native);
      if (!event)
        return false;
      [view scrollWheel:event];
      return true;
    } @catch (NSException *exception) {
      (void)exception;
      return false;
    }
  }
}
