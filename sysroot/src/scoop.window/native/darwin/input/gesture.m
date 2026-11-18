#import "../window.h"
#include <math.h>

static SWGesturePhase gesturePhase(NSEventPhase phase) {
  if (phase & NSEventPhaseCancelled)
    return SWGestureCancelled;
  if (phase & NSEventPhaseEnded)
    return SWGestureEnded;
  if (phase & (NSEventPhaseBegan | NSEventPhaseMayBegin))
    return SWGestureBegan;
  if (phase & (NSEventPhaseChanged | NSEventPhaseStationary))
    return SWGestureChanged;
  return SWGestureNone;
}

static SWGestureSequence sequence(SWView *view, SWGestureKind kind) {
  return kind == SWGestureMagnify ? view.magnifyInput : view.rotateInput;
}

static void setSequence(SWView *view, SWGestureKind kind,
                        SWGestureSequence value) {
  if (kind == SWGestureMagnify)
    view.magnifyInput = value;
  else
    view.rotateInput = value;
}

static void enqueue(SWView *view, SWGestureData data) {
  SWEvent *event = sw_event(view.owner, SWGesture);
  event.gesture = data;
  [view.owner.application enqueue:event];
}

static void cancel(SWView *view, SWGestureKind kind) {
  SWGestureSequence current = sequence(view, kind);
  if (!current.active)
    return;
  current.active = false;
  setSequence(view, kind, current);
  current.latest.phase = SWGestureCancelled;
  current.latest.delta_x = current.latest.delta_y = 0;
  enqueue(view, current.latest);
}

static void gesture(SWView *view, NSEvent *native, SWGestureKind kind) {
  SWGestureData data = {.kind = kind};
  if (kind != SWGestureSwipe)
    data.phase = gesturePhase(native.phase);
  NSPoint point = [view convertPoint:native.locationInWindow fromView:nil];
  data.x = point.x;
  data.y = point.y;
  data.scale = view.window.backingScaleFactor;
  data.physical_x = point.x * data.scale;
  data.physical_y = point.y * data.scale;
  data.modifiers = (int64_t)sw_input_modifiers(view, native.modifierFlags);
  if (kind == SWGestureMagnify)
    data.delta_x = native.magnification;
  else if (kind == SWGestureRotate)
    data.delta_x = -native.rotation * (M_PI / 180.0);
  else {
    // AppKit uses +1 for left/up. Window directions are right/down.
    data.delta_x = -native.deltaX;
    data.delta_y = -native.deltaY;
  }
  if (!isfinite(data.x) || !isfinite(data.y) || !isfinite(data.scale) ||
      data.scale <= 0 || !isfinite(data.physical_x) ||
      !isfinite(data.physical_y) || !isfinite(data.delta_x) ||
      !isfinite(data.delta_y) ||
      (kind == SWGestureMagnify && data.delta_x <= -1.0)) {
    if (kind != SWGestureSwipe)
      cancel(view, kind);
    return;
  }
  if (kind != SWGestureSwipe) {
    SWGestureSequence current = sequence(view, kind);
    if (data.phase == SWGestureBegan || data.phase == SWGestureNone) {
      cancel(view, kind);
      current.active = data.phase == SWGestureBegan;
    } else {
      // A late native change or end cannot restart a cancelled sequence.
      if (!current.active)
        return;
      if (data.phase == SWGestureEnded || data.phase == SWGestureCancelled)
        current.active = false;
    }
    if (data.phase == SWGestureCancelled)
      data.delta_x = data.delta_y = 0;
    current.latest = data;
    setSequence(view, kind, current);
  }
  sw_modifiers(view, native.modifierFlags);
  enqueue(view, data);
}

@implementation SWView (Gesture)
- (void)magnifyWithEvent:(NSEvent *)event {
  gesture(self, event, SWGestureMagnify);
}
- (void)rotateWithEvent:(NSEvent *)event {
  gesture(self, event, SWGestureRotate);
}
- (void)swipeWithEvent:(NSEvent *)event {
  gesture(self, event, SWGestureSwipe);
}
- (void)cancelGestures {
  cancel(self, SWGestureMagnify);
  cancel(self, SWGestureRotate);
}
@end
