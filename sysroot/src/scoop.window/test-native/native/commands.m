#import "../../native/darwin/window.h"
#import <objc/runtime.h>

static int mode;

static void interpret(SWView *view, SEL selector, NSArray<NSEvent *> *events) {
  NSRange absent = NSMakeRange(NSNotFound, 0);
  switch (mode) {
  case 1:
    [view doCommandBySelector:@selector(moveWordLeftAndModifySelection:)];
    [view doCommandBySelector:@selector(deleteWordForward:)];
    [view doCommandBySelector:@selector(insertNewline:)];
    break;
  case 2:
    [view
        doCommandBySelector:NSSelectorFromString(@"unknownScoopTestCommand:")];
    break;
  case 3:
    [view doCommandBySelector:@selector(insertNewline:)];
    [view insertText:@"中文" replacementRange:absent];
    [view doCommandBySelector:@selector(insertNewline:)];
    break;
  case 4:
    [view cancelComposition];
    [view doCommandBySelector:@selector(cancelOperation:)];
    break;
  case 5:
    [view setMarkedText:@"更新"
           selectedRange:NSMakeRange(2, 0)
        replacementRange:absent];
    [view doCommandBySelector:@selector(moveLeft:)];
    break;
  case 6:
    [view insertText:@"x" replacementRange:absent];
    [view
        doCommandBySelector:NSSelectorFromString(@"unknownScoopTestCommand:")];
    break;
  }
}

static void send_key(SWView *view, unsigned short code, NSString *text,
                     NSEventModifierFlags flags, BOOL repeat, BOOL release) {
  NSEvent *down = [NSEvent keyEventWithType:NSEventTypeKeyDown
                                   location:NSZeroPoint
                              modifierFlags:flags
                                  timestamp:1.0
                               windowNumber:view.window.windowNumber
                                    context:nil
                                 characters:text
                charactersIgnoringModifiers:text
                                  isARepeat:repeat
                                    keyCode:code];
  [view keyDown:down];
  if (release) {
    NSEvent *up = [NSEvent keyEventWithType:NSEventTypeKeyUp
                                   location:NSZeroPoint
                              modifierFlags:flags
                                  timestamp:1.1
                               windowNumber:view.window.windowNumber
                                    context:nil
                                 characters:text
                charactersIgnoringModifiers:text
                                  isARepeat:NO
                                    keyCode:code];
    [view keyUp:up];
  }
}

int32_t scoop_window_test_commands(void *pointer) {
  @autoreleasepool {
    SWView *view = (__bridge SWView *)pointer;
    if (!view.window.keyWindow || view.window.firstResponder != view ||
        !view.imeAllowed)
      return 1;
    @try {
      // First exercise the real system key binding interpreter.
      send_key(view, 123, @"\uf702", 0, NO, YES);
      send_key(view, 123, @"\uf702",
               NSEventModifierFlagShift | NSEventModifierFlagOption, NO, YES);

      // Reproduce ordered selector/IME callbacks inside the actual keyDown
      // transaction.
      Method method =
          class_getInstanceMethod(view.class, @selector(interpretKeyEvents:));
      if (!method)
        return 2;
      IMP original = method_setImplementation(method, (IMP)interpret);
      @try {
        NSRange absent = NSMakeRange(NSNotFound, 0);
        mode = 1;
        send_key(view, 0, @"a", 0, NO, YES);
        mode = 2;
        send_key(view, 1, @"s", 0, NO, YES);
        [view setMarkedText:@"中"
               selectedRange:NSMakeRange(1, 0)
            replacementRange:absent];
        mode = 3;
        send_key(view, 36, @"\r", 0, NO, YES);
        [view setMarkedText:@"かな"
               selectedRange:NSMakeRange(2, 0)
            replacementRange:absent];
        mode = 4;
        send_key(view, 53, @"\x1b", 0, NO, YES);
        [view setMarkedText:@"旧"
               selectedRange:NSMakeRange(1, 0)
            replacementRange:absent];
        mode = 5;
        send_key(view, 123, @"\uf702", 0, NO, YES);
        [view cancelComposition];
        mode = 6;
        send_key(view, 7, @"x", 0, NO, YES);
        mode = 1;
        send_key(view, 0, @"a", 0, NO, NO);
        send_key(view, 0, @"a", 0, YES, NO);
        [view resetInput];
      } @finally {
        method_setImplementation(method, original);
      }
      return 0;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}
