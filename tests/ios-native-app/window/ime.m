@import UIKit;
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

void probeTextStep(UIView *content, int32_t step) {
  UITextView *editor = nil;
  for (UIView *child in content.subviews)
    if ([child conformsToProtocol:@protocol(UITextInput)])
      editor = (UITextView *)child;
  assert(editor);
  if (step == 6) {
    assert(!editor.isFirstResponder && [editor.text isEqualToString:@"A🙂拟"]);
    puts("scoop-ios-ime-ok");
    fflush(stdout);
#if TARGET_OS_SIMULATOR
    exit(0);
#endif
    return;
  }
  assert(editor.isFirstResponder);
  CGRect caret =
      [content convertRect:[editor firstRectForRange:editor.selectedTextRange]
                  fromView:editor];
  assert(CGRectEqualToRect(caret, CGRectMake(37, 83, 2, 23)));
  switch (step) {
  case 0:
    assert([editor.text isEqualToString:@"A🙂"] &&
           NSEqualRanges(editor.selectedRange, NSMakeRange(3, 0)));
    [editor setMarkedText:@"ni" selectedRange:NSMakeRange(2, 0)];
    break;
  case 1:
    [editor setMarkedText:@"拟" selectedRange:NSMakeRange(1, 0)];
    break;
  case 2:
    [editor unmarkText];
    break;
  case 3: {
    UITextPosition *start =
        [editor positionFromPosition:editor.beginningOfDocument offset:1];
    UITextPosition *end = [editor positionFromPosition:start offset:2];
    editor.selectedTextRange = [editor textRangeFromPosition:start
                                                  toPosition:end];
    break;
  }
  case 4:
    [editor deleteBackward];
    break;
  case 5:
    [editor insertText:@"🙂"];
    break;
  default:
    abort();
  }
}
