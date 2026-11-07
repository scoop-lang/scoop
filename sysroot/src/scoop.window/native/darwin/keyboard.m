#import "window.h"

int32_t scoop_window_shortcut_equal(const uint8_t *left, int64_t left_size,
                                    const uint8_t *right, int64_t right_size) {
  @autoreleasepool {
    @try {
      NSString *a = [[NSString alloc] initWithBytes:left
                                             length:(NSUInteger)left_size
                                           encoding:NSUTF8StringEncoding];
      NSString *b = [[NSString alloc] initWithBytes:right
                                             length:(NSUInteger)right_size
                                           encoding:NSUTF8StringEncoding];
      if (!a || !b)
        return -1;
      NSMutableString *folded_a = [a mutableCopy];
      NSMutableString *folded_b = [b mutableCopy];
      if (!folded_a || !folded_b)
        return -1;
      CFStringFold((__bridge CFMutableStringRef)folded_a,
                   kCFCompareCaseInsensitive, CFLocaleGetSystem());
      CFStringFold((__bridge CFMutableStringRef)folded_b,
                   kCFCompareCaseInsensitive, CFLocaleGetSystem());
      CFStringNormalize((__bridge CFMutableStringRef)folded_a,
                        kCFStringNormalizationFormC);
      CFStringNormalize((__bridge CFMutableStringRef)folded_b,
                        kCFStringNormalizationFormC);
      return CFStringCompare((__bridge CFStringRef)folded_a,
                             (__bridge CFStringRef)folded_b,
                             0) == kCFCompareEqualTo;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}

NSEventModifierFlags sw_input_modifiers(SWView *view,
                                        NSEventModifierFlags flags) {
  // AppKit also sets Function on arrow/function-key events without a held Fn.
  return (flags & ~NSEventModifierFlagFunction) |
         (view.functionHeld ? NSEventModifierFlagFunction : 0);
}

void sw_modifiers(SWView *view, NSEventModifierFlags flags) {
  flags = sw_input_modifiers(view, flags);
  if (view.modifiers == flags)
    return;
  view.modifiers = flags;
  SWEvent *event = sw_event(view.owner, SWModifiers);
  SWEventData data = event.data;
  data.a = (int64_t)flags;
  event.data = data;
  [view.owner.application enqueue:event];
}

static void key(SWView *view, NSEvent *native, BOOL down) {
  NSNumber *code = @(native.keyCode);
  if (!down && !view.pressedKeys[code])
    return;
  SWEvent *event = sw_event(view.owner, SWKeyboard);
  SWEventData data = event.data;
  data.a = native.keyCode;
  data.b = (int64_t)sw_input_modifiers(view, native.modifierFlags);
  BOOL keyEvent =
      native.type == NSEventTypeKeyDown || native.type == NSEventTypeKeyUp;
  data.flags = down ? 1 | (keyEvent && native.isARepeat ? 2 : 0) : 0;
  if (keyEvent) {
    NSString *text = native.characters;
    if (!text.length || (native.modifierFlags & (NSEventModifierFlagControl |
                                                 NSEventModifierFlagCommand)))
      text =
          [native charactersByApplyingModifiers:native.modifierFlags &
                                                ~(NSEventModifierFlagControl |
                                                  NSEventModifierFlagCommand)];
    if (!native.characters.length && text.length)
      data.flags |= 8;
    event.text = [(text ?: native.charactersIgnoringModifiers)
        dataUsingEncoding:NSUTF8StringEncoding];
  }
  event.data = data;
  if (down && keyEvent)
    event.nativeInput = native;
  if (down && keyEvent)
    event.commands = [view.textCommands copy];
  if (down)
    view.pressedKeys[code] = event;
  else
    [view.pressedKeys removeObjectForKey:code];
  [view.owner.application enqueue:event];
}

@implementation SWView (Input)
- (void)keyDown:(NSEvent *)event {
  sw_modifiers(self, event.modifierFlags);
  BOOL shortcut = (event.modifierFlags & (NSEventModifierFlagControl |
                                          NSEventModifierFlagCommand)) != 0;
  [self beginTextBatch];
  @try {
    if (self.imeAllowed && !shortcut)
      [self interpretKeyEvents:@[ event ]];
    else if (!shortcut)
      sw_commit(self, event.characters);
    BOOL editedComposition = self.compositionConsumed &&
                             (self.textEdits.count || self.textBatchInvalid);
    if (shortcut ||
        (!editedComposition && (!self.compositionConsumed || self.forwardKey)))
      key(self, event, YES);
  } @finally {
    [self endTextBatch];
  }
}
- (void)keyUp:(NSEvent *)event {
  sw_modifiers(self, event.modifierFlags);
  key(self, event, NO);
}
- (BOOL)performKeyEquivalent:(NSEvent *)event {
  if (event.type == NSEventTypeKeyDown &&
      sw_menu_key_equivalent(self.owner.application, event))
    return YES;
  if (event.type != NSEventTypeKeyDown ||
      !(event.modifierFlags &
        (NSEventModifierFlagCommand | NSEventModifierFlagControl)))
    return NO;
  [self keyDown:event];
  return YES;
}
- (void)flagsChanged:(NSEvent *)event {
  if (event.keyCode == 63)
    self.functionHeld =
        (event.modifierFlags & NSEventModifierFlagFunction) != 0;
  sw_modifiers(self, event.modifierFlags);
  NSEventModifierFlags aggregate = 0, side = 0, pair = 0;
  switch (event.keyCode) {
  case 54:
    aggregate = NSEventModifierFlagCommand;
    side = 0x10;
    pair = 0x18;
    break;
  case 55:
    aggregate = NSEventModifierFlagCommand;
    side = 0x08;
    pair = 0x18;
    break;
  case 56:
    aggregate = NSEventModifierFlagShift;
    side = 0x02;
    pair = 0x06;
    break;
  case 60:
    aggregate = NSEventModifierFlagShift;
    side = 0x04;
    pair = 0x06;
    break;
  case 58:
    aggregate = NSEventModifierFlagOption;
    side = 0x20;
    pair = 0x60;
    break;
  case 61:
    aggregate = NSEventModifierFlagOption;
    side = 0x40;
    pair = 0x60;
    break;
  case 59:
    aggregate = NSEventModifierFlagControl;
    side = 0x01;
    pair = 0x2001;
    break;
  case 62:
    aggregate = NSEventModifierFlagControl;
    side = 0x2000;
    pair = 0x2001;
    break;
  case 63:
    aggregate = NSEventModifierFlagFunction;
    break;
  default:
    return;
  }
  BOOL held = self.pressedKeys[@(event.keyCode)] != nil;
  BOOL down = (event.modifierFlags & aggregate) != 0;
  if (down && pair)
    down = (event.modifierFlags & pair) ? (event.modifierFlags & side) != 0
                                        : !held;
  if (down != held)
    key(self, event, down);
}
- (void)resetInput {
  [self cancelPointer:0];
  [self cancelComposition];
  self.functionHeld = NO;
  NSEventModifierFlags locks = self.modifiers & NSEventModifierFlagCapsLock;
  for (NSNumber *code in [[self.pressedKeys allKeys]
           sortedArrayUsingSelector:@selector(compare:)]) {
    SWEvent *original = self.pressedKeys[code];
    SWEvent *event = sw_event(self.owner, SWKeyboard);
    SWEventData data = original.data;
    sw_stamp_input(&data);
    data.flags = 4;
    data.b = (int64_t)locks;
    event.data = data;
    event.text = original.text;
    [self.owner.application enqueue:event];
  }
  [self.pressedKeys removeAllObjects];
  sw_modifiers(self, locks);
  self.pointerInside = NO;
  [self updateCursor];
}
@end
