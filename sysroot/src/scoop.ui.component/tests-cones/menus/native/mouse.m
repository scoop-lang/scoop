#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

int32_t scoop_menu_test_right_click(void *pointer, double x, double y) {
  @autoreleasepool {
    @try {
      NSView *view = (__bridge NSView *)pointer;
      NSPoint location = [view convertPoint:NSMakePoint(x, y) toView:nil];
      CGFloat top = NSMaxY(NSScreen.screens.firstObject.frame);
      for (int pressed = 1; pressed >= 0; --pressed) {
        // NSEvent's mouse convenience constructor always uses button zero.
        // Direct view delivery needs locationInWindow, as in the scroll helper.
        CGEventRef native = CGEventCreateMouseEvent(
            NULL, pressed ? kCGEventRightMouseDown : kCGEventRightMouseUp,
            CGPointMake(location.x, top - location.y), kCGMouseButtonRight);
        if (!native)
          return -1;
        CGEventSetFlags(native, 0);
        CGEventSetIntegerValueField(native, kCGMouseEventClickState, 1);
        NSEvent *event = [NSEvent eventWithCGEvent:native];
        CFRelease(native);
        if (!event || event.buttonNumber != 1)
          return -1;
        if (pressed)
          [view rightMouseDown:event];
        else
          [view rightMouseUp:event];
      }
      return 0;
    } @catch (NSException *exception) {
      (void)exception;
      return -1;
    }
  }
}
