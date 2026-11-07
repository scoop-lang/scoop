#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

double scoop_test_frame_query(void *window_pointer, void *view_pointer,
                              int32_t field, double x, double y) {
  NSWindow *window = (__bridge NSWindow *)window_pointer;
  NSView *view = (__bridge NSView *)view_pointer;
  static __weak NSView *original;
  switch (field) {
  case 0:
    return !(window.styleMask & NSWindowStyleMaskTitled)               ? 2
           : (window.styleMask & NSWindowStyleMaskFullSizeContentView) ? 1
                                                                       : 0;
  case 1:
    if (!original)
      original = view;
    return original == view;
  case 4:
    return [view.layer isKindOfClass:CAMetalLayer.class] && view.layer.opaque;
  case 5: {
    NSPoint point = [view convertPoint:NSMakePoint(x, y) toView:nil];
    NSEvent *event =
        [NSEvent mouseEventWithType:NSEventTypeLeftMouseDown
                           location:point
                      modifierFlags:0
                          timestamp:NSProcessInfo.processInfo.systemUptime
                       windowNumber:window.windowNumber
                            context:nil
                        eventNumber:1
                         clickCount:1
                           pressure:1];
    [view mouseDown:event];
    return 1;
  }
  case 6:
    return view.layer.opaque;
  default:
    return -1;
  }
}
