#import "../../native/darwin/window.h"

static BOOL unavailable(SWView *view) {
  NSRange actual = NSMakeRange(0, 1);
  NSRect rect = [view firstRectForCharacterRange:NSMakeRange(NSNotFound, 0)
                                     actualRange:&actual];
  return NSEqualRects(rect, NSZeroRect) && actual.location == NSNotFound &&
         [view characterIndexForPoint:NSMakePoint(1, 1)] == NSNotFound;
}

static BOOL document(SWView *view, NSString *expected, NSRange selection) {
  NSRange requested = NSMakeRange(0, expected.length);
  NSRange actual = NSMakeRange(NSNotFound, 0);
  NSAttributedString *text = [view attributedSubstringForProposedRange:requested
                                                           actualRange:&actual];
  return text && [text.string isEqualToString:expected] &&
         NSEqualRanges(requested, actual) &&
         NSEqualRanges(view.selectedRange, selection);
}

static BOOL caret(SWView *view, NSUInteger cursor) {
  NSRange actual = NSMakeRange(NSNotFound, 0);
  NSRect rect = [view firstRectForCharacterRange:NSMakeRange(cursor, 0)
                                     actualRange:&actual];
  NSRect expected = [view.window
      convertRectToScreen:[view convertRect:NSMakeRect(12, 34, 2, 18)
                                     toView:nil]];
  return NSEqualRects(rect, expected) &&
         NSEqualRanges(actual, NSMakeRange(cursor, 0)) &&
         [view characterIndexForPoint:NSMakePoint(NSMidX(rect),
                                                  NSMidY(rect))] == cursor &&
         [view characterIndexForPoint:NSMakePoint(NSMaxX(rect) + 10,
                                                  NSMidY(rect))] == NSNotFound;
}

int32_t scoop_window_test_snapshot(void *pointer, int32_t phase) {
  @autoreleasepool {
    @try {
      SWView *view = (__bridge SWView *)pointer;
      NSRange absent = NSMakeRange(NSNotFound, 0);
      NSRange actual = absent;
      switch (phase) {
      case 0:
        if (!document(view, @"A😀中BC\nD", NSMakeRange(3, 3)) ||
            !caret(view, 3))
          return 10;
        if ([view attributedSubstringForProposedRange:NSMakeRange(2, 1)
                                          actualRange:&actual] ||
            actual.location != NSNotFound ||
            [view attributedSubstringForProposedRange:NSMakeRange(
                                                          NSUIntegerMax - 1, 4)
                                          actualRange:&actual])
          return 11;
        if (!NSEqualRects([view firstRectForCharacterRange:NSMakeRange(0, 1)
                                               actualRange:&actual],
                          NSZeroRect) ||
            actual.location != NSNotFound)
          return 12;
        break;
      case 1:
        [view beginTextBatch];
        [view setMarkedText:@"仮😀"
               selectedRange:NSMakeRange(1, 2)
            replacementRange:NSMakeRange(3, 3)];
        if (!document(view, @"A😀仮😀\nD", NSMakeRange(4, 2)) ||
            !NSEqualRanges(view.markedRange, NSMakeRange(3, 3))) {
          [view endTextBatch];
          return 20;
        }
        [view insertText:@"文" replacementRange:NSMakeRange(3, 3)];
        [view endTextBatch];
        [view setMarkedText:@"日本"
               selectedRange:NSMakeRange(2, 0)
            replacementRange:absent];
        [view setMarkedText:@"日本語"
               selectedRange:NSMakeRange(3, 0)
            replacementRange:absent];
        [view unmarkText];
        if ([view hasMarkedText] || !unavailable(view) ||
            [view attributedSubstringForProposedRange:NSMakeRange(0, 1)
                                          actualRange:&actual])
          return 21;
        break;
      case 2:
        if (!document(view, @"A😀文日本語\nD", NSMakeRange(7, 0)) ||
            [view hasMarkedText] || !caret(view, 7))
          return 30;
        break;
      case 3:
        [view setMarkedText:@"候補"
               selectedRange:NSMakeRange(2, 0)
            replacementRange:absent];
        [view beginTextBatch];
        [view insertText:@"BAD" replacementRange:NSMakeRange(0, 1)];
        [view endTextBatch];
        if ([view hasMarkedText] || !unavailable(view))
          return 40;
        break;
      case 4:
        [view insertText:@"R" replacementRange:NSMakeRange(0, 1)];
        break;
      case 5:
        if (!document(view, @"external😀", NSMakeRange(10, 0)) ||
            !caret(view, 10))
          return 50;
        break;
      case 6:
        if (!unavailable(view) ||
            [view attributedSubstringForProposedRange:NSMakeRange(0, 1)
                                          actualRange:&actual])
          return 60;
        break;
      case 7:
        if (!unavailable(view))
          return 70;
        break;
      default:
        return 1;
      }
      return 0;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}
