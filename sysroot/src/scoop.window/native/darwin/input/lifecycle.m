#import "../window.h"

@implementation SWView (Input)
- (void)installInput {
  self.pressedKeys = [NSMutableDictionary new];
  self.pressedButtons = [NSMutableSet new];
  self.cursor = NSCursor.arrowCursor;
  self.cursorVisible = YES;
}
- (void)activateInput {
  self.functionHeld =
      (NSEvent.modifierFlags & NSEventModifierFlagFunction) != 0;
  if (self.imeAllowed)
    [self.inputContext activate];
  NSPoint point =
      [self convertPoint:self.window.mouseLocationOutsideOfEventStream
                fromView:nil];
  self.pointerInside = NSMouseInRect(point, self.bounds, self.flipped);
  [self updateCursor];
}
- (void)deactivateInput {
  [self resetInput];
  if (self.imeAllowed)
    [self.inputContext deactivate];
}
- (void)resetInput {
  [self cancelPointer:SWPointerFocusLost];
  [self cancelComposition];
  self.functionHeld = NO;
  NSEventModifierFlags locks = self.modifiers & NSEventModifierFlagCapsLock;
  for (NSNumber *code in [[self.pressedKeys allKeys]
           sortedArrayUsingSelector:@selector(compare:)]) {
    SWEvent *original = self.pressedKeys[code];
    SWEvent *event = sw_event(self.owner, SWKeyboard);
    SWKeyboardData data = original.keyboard;
    data.pressed = 0;
    data.repeat = 0;
    data.synthetic = 1;
    data.modifiers = (int64_t)locks;
    event.keyboard = data;
    event.text = original.text;
    [self.owner.application enqueue:event];
  }
  [self.pressedKeys removeAllObjects];
  sw_modifiers(self, locks);
  self.pointerInside = NO;
  [self updateCursor];
}
@end
