#import "../../native/darwin/window.h"

// AppKit has no public factory for gesture NSEvents. Controlled properties
// exercise the production responder and queue without claiming device input.
@interface SWTestGestureEvent : NSEvent
@property(nonatomic) NSEventType testType;
@property(nonatomic) NSEventPhase testPhase;
@property(nonatomic) NSPoint testPosition;
@property(nonatomic) double testX, testY;
@end

@implementation SWTestGestureEvent
- (NSEventType)type {
  return self.testType;
}
- (NSEventPhase)phase {
  return self.testPhase;
}
- (NSPoint)locationInWindow {
  return self.testPosition;
}
- (NSEventModifierFlags)modifierFlags {
  return NSEventModifierFlagShift;
}
- (CGFloat)magnification {
  return self.testX;
}
- (float)rotation {
  return (float)self.testX;
}
- (CGFloat)deltaX {
  return self.testX;
}
- (CGFloat)deltaY {
  return self.testY;
}
@end

void scoop_window_test_gesture(void *pointer, int32_t kind, int32_t phase,
                               double x, double y, double delta_x,
                               double delta_y) {
  SWView *view = (__bridge SWView *)pointer;
  SWTestGestureEvent *event = [SWTestGestureEvent new];
  const NSEventPhase phases[] = {NSEventPhaseNone, NSEventPhaseBegan,
                                 NSEventPhaseChanged, NSEventPhaseEnded,
                                 NSEventPhaseCancelled};
  event.testPhase = phases[phase];
  event.testPosition = [view convertPoint:NSMakePoint(x, y) toView:nil];
  event.testX = delta_x;
  event.testY = delta_y;
  if (kind == 0) {
    event.testType = NSEventTypeMagnify;
    [view magnifyWithEvent:event];
  } else if (kind == 1) {
    event.testType = NSEventTypeRotate;
    [view rotateWithEvent:event];
  } else {
    event.testType = NSEventTypeSwipe;
    [view swipeWithEvent:event];
  }
}

void scoop_window_test_cancel_gestures(void *pointer) {
  [(__bridge SWView *)pointer cancelPointer:SWPointerCaptureLost];
}
