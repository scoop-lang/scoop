#import <AppKit/AppKit.h>
#include <stdint.h>

typedef struct {
  int64_t start, length;
} SWTestTextRange;
typedef struct {
  double x, y, width, height;
} SWTestTextBounds;

static SWTestTextRange outputRange(NSRange value) {
  return (SWTestTextRange){
      value.location == NSNotFound ? -1 : (int64_t)value.location,
      (int64_t)value.length};
}

SWTestTextRange scoop_ui_ax_range_for_index(void *pointer, int64_t index) {
  return outputRange([(__bridge id)pointer accessibilityRangeForIndex:index]);
}

SWTestTextRange scoop_ui_ax_range_for_line(void *pointer, int64_t line) {
  return outputRange([(__bridge id)pointer accessibilityRangeForLine:line]);
}

SWTestTextRange scoop_ui_ax_visible_range(void *pointer) {
  return outputRange([(__bridge id)pointer accessibilityVisibleCharacterRange]);
}

int64_t scoop_ui_ax_line_for_index(void *pointer, int64_t index) {
  return [(__bridge id)pointer accessibilityLineForIndex:index];
}

int64_t scoop_ui_ax_insertion_line(void *pointer) {
  return [(__bridge id)pointer accessibilityInsertionPointLineNumber];
}

SWTestTextRange scoop_ui_ax_range_at_point(void *pointer, void *viewPointer,
                                           double x, double y) {
  NSView *view = (__bridge NSView *)viewPointer;
  NSPoint window = [view convertPoint:NSMakePoint(x, y) toView:nil];
  NSPoint screen = [view.window convertPointToScreen:window];
  return outputRange(
      [(__bridge id)pointer accessibilityRangeForPosition:screen]);
}

SWTestTextBounds scoop_ui_ax_bounds_for_range(void *pointer, void *viewPointer,
                                              int64_t start, int64_t length) {
  NSView *view = (__bridge NSView *)viewPointer;
  NSRect screen = [(__bridge id)pointer
      accessibilityFrameForRange:NSMakeRange(start, length)];
  NSRect window = [view.window convertRectFromScreen:screen];
  NSRect rect = [view convertRect:window fromView:nil];
  return (SWTestTextBounds){rect.origin.x, rect.origin.y, rect.size.width,
                            rect.size.height};
}

int32_t scoop_ui_ax_empty_text_bounds(void *pointer, int64_t start,
                                      int64_t length) {
  return NSEqualRects(
      [(__bridge id)pointer
          accessibilityFrameForRange:NSMakeRange(start, length)],
      NSZeroRect);
}

int32_t scoop_ui_ax_has_text_geometry(void *pointer) {
  id node = (__bridge id)pointer;
  return
      [node isAccessibilitySelectorAllowed:@selector(
                                               accessibilityRangeForIndex:)] &&
      [node isAccessibilitySelectorAllowed:@selector(
                                               accessibilityRangeForLine:)] &&
      [node isAccessibilitySelectorAllowed:@selector(
                                               accessibilityLineForIndex:)] &&
      [node
          isAccessibilitySelectorAllowed:@selector(
                                             accessibilityRangeForPosition:)] &&
      [node isAccessibilitySelectorAllowed:@selector(
                                               accessibilityFrameForRange:)] &&
      [node isAccessibilitySelectorAllowed:
                @selector(accessibilityVisibleCharacterRange)];
}
