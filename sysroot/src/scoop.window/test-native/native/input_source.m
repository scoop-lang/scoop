#import "../../native/darwin/window.h"
#import <Carbon/Carbon.h>

void *scoop_window_test_save_input_source(void) {
  return (void *)TISCopyCurrentKeyboardInputSource();
}

int32_t scoop_window_test_restore_input_source(void *pointer) {
  if (!pointer)
    return 1;
  TISInputSourceRef source = (TISInputSourceRef)pointer;
  OSStatus status = TISSelectInputSource(source);
  CFRelease(source);
  return status;
}

int32_t scoop_window_test_select_pinyin(void *pointer) {
  @autoreleasepool {
    @try {
      SWView *view = (__bridge SWView *)pointer;
      if (!view.imeAllowed || !view.window.isKeyWindow)
        return 1;
      NSDictionary *filter = @{
        (__bridge NSString *)
        kTISPropertyInputSourceID : @"com.apple.inputmethod.SCIM.ITABC"
      };
      CFArrayRef sources =
          TISCreateInputSourceList((__bridge CFDictionaryRef)filter, false);
      if (!sources)
        return 2;
      OSStatus status = -1;
      if (CFArrayGetCount(sources) > 0)
        status = TISSelectInputSource(
            (TISInputSourceRef)CFArrayGetValueAtIndex(sources, 0));
      CFRelease(sources);
      if (status != noErr)
        return status;
      NSTextInputContext *context = view.inputContext;
      context.selectedKeyboardInputSource = @"com.apple.inputmethod.SCIM.ITABC";
      [context activate];
      return 0;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}

bool scoop_window_test_pinyin_selected(void *pointer) {
  @autoreleasepool {
    SWView *view = (__bridge SWView *)pointer;
    TISInputSourceRef source = TISCopyCurrentKeyboardInputSource();
    if (!source)
      return false;
    CFStringRef identifier =
        TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
    bool selected =
        identifier &&
        CFEqual(identifier, CFSTR("com.apple.inputmethod.SCIM.ITABC"));
    CFRelease(source);
    return selected && [view.inputContext.selectedKeyboardInputSource
                           isEqualToString:@"com.apple.inputmethod.SCIM.ITABC"];
  }
}

int32_t scoop_window_test_ime_key(void *pointer, int32_t code) {
  @autoreleasepool {
    @try {
      SWView *view = (__bridge SWView *)pointer;
      if (!view.imeAllowed || !view.window.isKeyWindow || code < 0 ||
          code > UINT16_MAX)
        return 1;
      for (int released = 0; released < 2; ++released) {
        // String-only NSEvents bypass the system input method's key handling.
        CGEventRef native =
            CGEventCreateKeyboardEvent(NULL, (CGKeyCode)code, !released);
        if (!native)
          return 2;
        @try {
          CGEventSetFlags(native, 0);
          NSEvent *event = [NSEvent eventWithCGEvent:native];
          if (!event)
            return 2;
          if (released)
            [view keyUp:event];
          else
            [view keyDown:event];
        } @finally {
          CFRelease(native);
        }
      }
      return 0;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}
