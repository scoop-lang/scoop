#import "../window.h"
#include <limits.h>

static SWScrollPhase scrollPhase(NSEventPhase phase) {
  if (phase & NSEventPhaseCancelled)
    return SWScrollCancelled;
  if (phase & NSEventPhaseEnded)
    return SWScrollEnded;
  if (phase & NSEventPhaseBegan)
    return SWScrollBegan;
  if (phase & NSEventPhaseMayBegin)
    return SWScrollMayBegin;
  if (phase & NSEventPhaseChanged)
    return SWScrollChanged;
  if (phase & NSEventPhaseStationary)
    return SWScrollStationary;
  return SWScrollNone;
}

static void pointer(SWView *view, NSEvent *native, SWPointerKind kind) {
  sw_modifiers(view, native.modifierFlags);
  if (kind == SWPointerPressed || kind == SWPointerReleased) {
    NSNumber *button = @(native.buttonNumber);
    if (kind == SWPointerReleased &&
        ![view.pressedButtons containsObject:button])
      return;
    if (kind == SWPointerPressed)
      [view.pressedButtons addObject:button];
    else
      [view.pressedButtons removeObject:button];
  }
  SWEvent *event = sw_event(view.owner, SWPointer);
  SWPointerData data = {.kind = kind};
  NSPoint logical = [view convertPoint:native.locationInWindow fromView:nil];
  data.x = logical.x;
  data.y = logical.y;
  data.scale = view.window.backingScaleFactor;
  // Backing surface points may use a different origin; both payload positions
  // use the content view's top-left origin and the scale from this event.
  data.physical_x = logical.x * data.scale;
  data.physical_y = logical.y * data.scale;
  if (kind == SWPointerMoved) {
    view.pointerInside = NSMouseInRect(logical, view.bounds, view.flipped);
    [view updateCursor];
  }
  data.modifiers = (int64_t)sw_input_modifiers(view, native.modifierFlags);
  if (kind == SWPointerPressed || kind == SWPointerReleased) {
    data.button = (int32_t)native.buttonNumber;
    data.click_count = (int32_t)MAX(1, MIN(INT_MAX, native.clickCount));
  }
  if (kind == SWPointerLineWheel || kind == SWPointerPixelWheel) {
    sw_pointer_scroll(&data, kind == SWPointerPixelWheel,
                      native.scrollingDeltaX, native.scrollingDeltaY,
                      scrollPhase(native.phase),
                      scrollPhase(native.momentumPhase));
  }
  event.pointer = data;
  if (kind == SWPointerPressed ||
      (kind == SWPointerMoved && native.type == NSEventTypeLeftMouseDragged &&
       [view.pressedButtons containsObject:@0]))
    event.nativeInput = native;
  [view.owner.application enqueue:event];
}

@implementation SWView (Pointer)
- (void)cancelPointer:(int32_t)reason {
  [self cancelGestures];
  if (!self.pressedButtons.count)
    return;
  [self.pressedButtons removeAllObjects];
  SWEvent *event = sw_event(self.owner, SWPointer);
  event.pointer =
      (SWPointerData){.kind = SWPointerCancelled, .cancel_reason = reason};
  [self.owner.application enqueue:event];
}
- (void)updateTrackingAreas {
  [super updateTrackingAreas];
  if (self.tracking)
    [self removeTrackingArea:self.tracking];
  self.tracking = [[NSTrackingArea alloc]
      initWithRect:NSZeroRect
           options:NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved |
                   NSTrackingActiveAlways | NSTrackingInVisibleRect |
                   NSTrackingCursorUpdate
             owner:self
          userInfo:nil];
  [self addTrackingArea:self.tracking];
}
- (void)resetCursorRects {
  [self addCursorRect:self.bounds cursor:self.cursor ?: NSCursor.arrowCursor];
}
- (void)cursorUpdate:(NSEvent *)event {
  [self updateCursor];
}
- (void)updateCursor {
  BOOL hidden =
      self.pointerInside && !self.cursorVisible && self.window.keyWindow;
  if (hidden != self.cursorHidden) {
    if (hidden)
      [NSCursor hide];
    else
      [NSCursor unhide];
    self.cursorHidden = hidden;
  }
  if (self.pointerInside)
    [self.cursor set];
}
- (void)mouseEntered:(NSEvent *)event {
  self.pointerInside = YES;
  [self updateCursor];
  pointer(self, event, SWPointerEntered);
}
- (void)mouseExited:(NSEvent *)event {
  self.pointerInside = NO;
  [self updateCursor];
  pointer(self, event, SWPointerLeft);
}
- (void)mouseMoved:(NSEvent *)event {
  pointer(self, event, SWPointerMoved);
}
- (void)mouseDragged:(NSEvent *)event {
  pointer(self, event, SWPointerMoved);
}
- (void)rightMouseDragged:(NSEvent *)event {
  pointer(self, event, SWPointerMoved);
}
- (void)otherMouseDragged:(NSEvent *)event {
  pointer(self, event, SWPointerMoved);
}
- (void)mouseDown:(NSEvent *)event {
  if (sw_frame_pointer(self, event))
    return;
  pointer(self, event, SWPointerPressed);
}
- (void)rightMouseDown:(NSEvent *)event {
  pointer(self, event, SWPointerPressed);
}
- (void)otherMouseDown:(NSEvent *)event {
  pointer(self, event, SWPointerPressed);
}
- (void)mouseUp:(NSEvent *)event {
  pointer(self, event, SWPointerReleased);
}
- (void)rightMouseUp:(NSEvent *)event {
  pointer(self, event, SWPointerReleased);
}
- (void)otherMouseUp:(NSEvent *)event {
  pointer(self, event, SWPointerReleased);
}
- (void)scrollWheel:(NSEvent *)event {
  pointer(self, event,
          event.hasPreciseScrollingDeltas ? SWPointerPixelWheel
                                          : SWPointerLineWheel);
}
@end

int64_t scoop_window_cursor_icon(void *pointer, int32_t icon) {
  SWView *view = ((__bridge SWWindow *)pointer).view;
  switch (icon) {
  case 1:
    view.cursor = NSCursor.IBeamCursor;
    break;
  case 2:
    view.cursor = NSCursor.crosshairCursor;
    break;
  case 3:
    view.cursor = NSCursor.pointingHandCursor;
    break;
  case 4:
    view.cursor = NSCursor.openHandCursor;
    break;
  case 5:
    view.cursor = NSCursor.closedHandCursor;
    break;
  case 6:
    view.cursor = NSCursor.operationNotAllowedCursor;
    break;
  case 7:
    view.cursor = NSCursor.resizeLeftRightCursor;
    break;
  case 8:
    view.cursor = NSCursor.resizeUpDownCursor;
    break;
  default:
    view.cursor = NSCursor.arrowCursor;
    break;
  }
  [view.window invalidateCursorRectsForView:view];
  [view updateCursor];
  return 0;
}

int64_t scoop_window_cursor_visible(void *pointer, int32_t visible) {
  SWView *view = ((__bridge SWWindow *)pointer).view;
  view.cursorVisible = visible != 0;
  [view updateCursor];
  return 0;
}
