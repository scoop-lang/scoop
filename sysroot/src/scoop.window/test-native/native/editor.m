#import "../../native/darwin/window.h"
#import <objc/runtime.h>

static NSString *string(const uint8_t *bytes, int64_t length) {
  return [[NSString alloc] initWithBytes:bytes
                                  length:(NSUInteger)length
                                encoding:NSUTF8StringEncoding];
}

int32_t scoop_window_test_editor_text(void *pointer, int32_t kind,
                                      const uint8_t *bytes, int64_t length,
                                      int64_t start, int64_t count,
                                      int64_t replacement, int64_t replaced) {
  @autoreleasepool {
    @try {
      SWView *view = (__bridge SWView *)pointer;
      if (!view.imeAllowed)
        return 1;
      NSString *text = string(bytes, length);
      if (!text)
        return 2;
      NSRange range = replacement < 0 ? NSMakeRange(NSNotFound, 0)
                                      : NSMakeRange(replacement, replaced);
      switch (kind) {
      case 0:
        [view insertText:text replacementRange:range];
        break;
      case 1:
        [view setMarkedText:text
               selectedRange:NSMakeRange(start, count)
            replacementRange:range];
        break;
      case 2:
        [view cancelComposition];
        break;
      case 3:
        [view unmarkText];
        break;
      default:
        return 3;
      }
      return 0;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}

static NSRange byte_range(const uint8_t *bytes, int64_t first, int64_t end) {
  if (first < 0)
    return NSMakeRange(NSNotFound, 0);
  NSUInteger start = string(bytes, first).length;
  return NSMakeRange(start, string(bytes, end).length - start);
}

bool scoop_window_test_editor_document(void *pointer, const uint8_t *bytes,
                                       int64_t length, int64_t first,
                                       int64_t end, int64_t marked,
                                       int64_t markedEnd) {
  @autoreleasepool {
    @try {
      SWView *view = (__bridge SWView *)pointer;
      NSString *expected = string(bytes, length);
      if (!expected ||
          !NSEqualRanges(view.selectedRange, byte_range(bytes, first, end)) ||
          !NSEqualRanges(view.markedRange,
                         byte_range(bytes, marked, markedEnd)))
        return false;
      if (expected.length == 0)
        return true;
      NSRange requested = NSMakeRange(0, expected.length);
      NSRange actual = NSMakeRange(NSNotFound, 0);
      NSAttributedString *value =
          [view attributedSubstringForProposedRange:requested
                                        actualRange:&actual];
      return value && [value.string isEqualToString:expected] &&
             NSEqualRanges(actual, requested);
    } @catch (NSException *exception) {
      return false;
    }
  }
}

static void editor_send_key(SWView *view, int32_t code, NSString *text,
                            int32_t flags) {
  NSEventModifierFlags modifiers =
      (flags & 1 ? NSEventModifierFlagShift : 0) |
      (flags & 2 ? NSEventModifierFlagControl : 0) |
      (flags & 4 ? NSEventModifierFlagOption : 0) |
      (flags & 8 ? NSEventModifierFlagCommand : 0);
  for (int released = 0; released < 2; ++released) {
    NSEvent *event =
        [NSEvent keyEventWithType:released ? NSEventTypeKeyUp
                                           : NSEventTypeKeyDown
                               location:NSZeroPoint
                          modifierFlags:modifiers
                              timestamp:NSProcessInfo.processInfo.systemUptime
                           windowNumber:view.window.windowNumber
                                context:nil
                             characters:text
            charactersIgnoringModifiers:text
                              isARepeat:NO
                                keyCode:(unsigned short)code];
    if (released)
      [view keyUp:event];
    else
      [view keyDown:event];
  }
}

int32_t scoop_window_test_editor_key(void *pointer, int32_t code,
                                     const uint8_t *bytes, int64_t length,
                                     int32_t flags) {
  @autoreleasepool {
    @try {
      editor_send_key((__bridge SWView *)pointer, code, string(bytes, length),
                      flags);
      return 0;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}

static int interpretation;
static void interpret(SWView *view, SEL selector, NSArray<NSEvent *> *events) {
  NSRange absent = NSMakeRange(NSNotFound, 0);
  switch (interpretation) {
  case 1:
    [view doCommandBySelector:@selector(insertNewline:)];
    [view insertText:@"中文" replacementRange:absent];
    [view doCommandBySelector:@selector(insertNewline:)];
    break;
  case 2:
    [view cancelComposition];
    [view doCommandBySelector:@selector(cancelOperation:)];
    break;
  case 3:
    [view doCommandBySelector:@selector(moveToBeginningOfDocument:)];
    [view doCommandBySelector:@selector(moveForwardAndModifySelection:)];
    [view doCommandBySelector:@selector(deleteBackward:)];
    break;
  case 4:
    [view insertText:@"x" replacementRange:absent];
    [view
        doCommandBySelector:NSSelectorFromString(@"unknownEditorTestCommand:")];
    break;
  }
}

int32_t scoop_window_test_editor_interpret(void *pointer, int32_t scenario) {
  @autoreleasepool {
    SWView *view = (__bridge SWView *)pointer;
    Method method =
        class_getInstanceMethod(view.class, @selector(interpretKeyEvents:));
    if (!method || scenario < 1 || scenario > 4)
      return 1;
    IMP original = method_setImplementation(method, (IMP)interpret);
    @try {
      interpretation = scenario;
      editor_send_key(view,
                      scenario == 1   ? 36
                      : scenario == 2 ? 53
                      : scenario == 3 ? 124
                                      : 0,
                      scenario == 1   ? @"\r"
                      : scenario == 2 ? @"\x1b"
                      : scenario == 3 ? @"\uf703"
                                      : @"a",
                      0);
      return 0;
    } @catch (NSException *exception) {
      return -1;
    } @finally {
      method_setImplementation(method, original);
    }
  }
}

int32_t scoop_window_test_editor_pointer(void *pointer, double x, double y,
                                         bool pressed, int32_t clicks,
                                         bool shift) {
  @autoreleasepool {
    @try {
      SWView *view = (__bridge SWView *)pointer;
      NSEvent *event = [NSEvent
          mouseEventWithType:pressed ? NSEventTypeLeftMouseDown
                                     : NSEventTypeLeftMouseUp
                    location:[view convertPoint:NSMakePoint(x, y) toView:nil]
               modifierFlags:shift ? NSEventModifierFlagShift : 0
                   timestamp:NSProcessInfo.processInfo.systemUptime
                windowNumber:view.window.windowNumber
                     context:nil
                 eventNumber:1
                  clickCount:clicks
                    pressure:pressed ? 1.0 : 0.0];
      if (pressed)
        [view mouseDown:event];
      else
        [view mouseUp:event];
      return 0;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}
