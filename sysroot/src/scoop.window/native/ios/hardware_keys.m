#import "text.h"

@implementation SWIScene (HardwareKeys)
- (void)updateModifiers:(int64_t)modifiers {
  if (modifiers == self.keyModifiers)
    return;
  self.keyModifiers = modifiers;
  SWIEvent *event = [SWIEvent new];
  event.header = swi_input_header(self, SWModifiers);
  event.data = (SWEventData){.a = modifiers};
  [self.application enqueue:event];
}
- (int64_t)currentModifiers:(UIKeyModifierFlags)flags {
  // Preserve sides only for modifiers whose physical presses UIKit delivered.
  static const int64_t sides[] = {1, 2, 32, 8, 8192, 4, 64, 16};
  int64_t result = flags;
  for (NSNumber *code in self.pressedKeys) {
    NSInteger offset =
        code.integerValue - UIKeyboardHIDUsageKeyboardLeftControl;
    if (offset >= 0 && offset < 8)
      result |= sides[offset];
  }
  return result;
}
- (void)deliverPress:(UIPress *)press composing:(BOOL)composing {
  UIKey *key = press.key;
  if (!key || (press.window && press.window != self.window))
    return;
  BOOL down = press.phase == UIPressPhaseBegan;
  BOOL cancelled = press.phase == UIPressPhaseCancelled;
  if (!down && !cancelled && press.phase != UIPressPhaseEnded)
    return;
  NSNumber *code = @(key.keyCode);
  SWIEvent *previous = self.pressedKeys[code];
  if (!down && !previous)
    return;
  UIKeyModifierFlags flags = key.modifierFlags;
  if (down && composing &&
      !(flags & (UIKeyModifierCommand | UIKeyModifierControl)))
    return;
  SWIEvent *event = [SWIEvent new];
  event.header = swi_input_header(self, SWKeyboard);
  SWKeyboardData data = {.physical_code = key.keyCode,
                         .pressed = down,
                         .repeat = down && previous,
                         .synthetic = cancelled};
  if (down) {
    NSString *characters = key.characters;
    if (flags & (UIKeyModifierCommand | UIKeyModifierControl))
      characters = key.charactersIgnoringModifiers;
    else if (!characters.length && key.charactersIgnoringModifiers.length) {
      characters = key.charactersIgnoringModifiers;
      data.dead = 1;
    }
    event.keyText = [characters dataUsingEncoding:NSUTF8StringEncoding];
    self.pressedKeys[code] = event;
  } else {
    event.keyText = previous.keyText;
    data.dead = previous.keyboard.dead;
    [self.pressedKeys removeObjectForKey:code];
  }
  data.modifiers = [self currentModifiers:flags];
  [self updateModifiers:data.modifiers];
  event.keyboard = data;
  event.data = (SWEventData){.flags = self.view.editor.isFirstResponder};
  [self.application enqueue:event];
}
- (void)deliverPresses:(NSSet<UIPress *> *)presses composing:(BOOL)composing {
  if (!self.window.isKeyWindow)
    return;
  if (!self.pressedKeys)
    self.pressedKeys = [NSMutableDictionary new];
  for (UIPress *press in presses)
    [self deliverPress:press composing:composing];
}
- (void)cancelKeys {
  int64_t flags = self.keyModifiers & UIKeyModifierAlphaShift;
  for (SWIEvent *previous in self.pressedKeys.allValues) {
    SWIEvent *event = [SWIEvent new];
    event.header = swi_input_header(self, SWKeyboard);
    event.keyText = previous.keyText;
    SWKeyboardData data = previous.keyboard;
    data.modifiers = flags;
    data.pressed = data.repeat = 0;
    data.synthetic = 1;
    event.keyboard = data;
    [self.application enqueue:event];
  }
  [self.pressedKeys removeAllObjects];
  [self updateModifiers:flags];
}
@end

SWKeyboardData scoop_window_event_keyboard(void *pointer) {
  SWIEvent *event = (__bridge SWIEvent *)pointer;
  SWKeyboardData data = event.keyboard;
  data.text = event.keyText.bytes;
  data.text_length = (int64_t)event.keyText.length;
  return data;
}
int64_t scoop_window_event_modifiers(void *pointer) {
  return ((__bridge SWIEvent *)pointer).data.a;
}
