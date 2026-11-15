#import <AppKit/AppKit.h>
#include <stdbool.h>
#include <stdint.h>

int32_t scoop_test_editor_context_click(void *pointer, double x, double y,
                                        bool control) {
  NSView *view = (__bridge NSView *)pointer;
  for (int release = 0; release != 2; ++release) {
    NSEventType type =
        control
            ? (release ? NSEventTypeLeftMouseUp : NSEventTypeLeftMouseDown)
            : (release ? NSEventTypeRightMouseUp : NSEventTypeRightMouseDown);
    NSEvent *event = [NSEvent
        mouseEventWithType:type
                  location:[view convertPoint:NSMakePoint(x, y) toView:nil]
             modifierFlags:control ? NSEventModifierFlagControl : 0
                 timestamp:NSProcessInfo.processInfo.systemUptime
              windowNumber:view.window.windowNumber
                   context:nil
               eventNumber:1
                clickCount:1
                  pressure:release ? 0.0 : 1.0];
    if (control) {
      if (release)
        [view mouseUp:event];
      else
        [view mouseDown:event];
    } else {
      if (release)
        [view rightMouseUp:event];
      else
        [view rightMouseDown:event];
    }
  }
  return 0;
}
