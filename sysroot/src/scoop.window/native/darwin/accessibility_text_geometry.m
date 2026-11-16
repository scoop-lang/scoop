#import "accessibility.h"
#include <math.h>

static NSRange unavailableRange(void) { return NSMakeRange(NSNotFound, 0); }

static BOOL textOffset(NSString *text, NSInteger index, int64_t *offset) {
  if (!text || index < 0 ||
      !sw_text_range(text, NSMakeRange((NSUInteger)index, 0)))
    return NO;
  *offset = (int64_t)[[text substringToIndex:(NSUInteger)index]
      lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
  return YES;
}

static NSRange textRange(NSString *text, SWAccessibilityTextRange range) {
  if (!text || range.status != 1 || range.start < 0 || range.end < range.start)
    return unavailableRange();
  const uint8_t *bytes = (const uint8_t *)text.UTF8String;
  NSUInteger length = [text lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
  if (!bytes || (uint64_t)range.end > length ||
      ((uint64_t)range.start < length && (bytes[range.start] & 0xc0) == 0x80) ||
      ((uint64_t)range.end < length && (bytes[range.end] & 0xc0) == 0x80))
    return unavailableRange();
  NSUInteger start = sw_text_utf16_offset(bytes, range.start);
  NSUInteger end = sw_text_utf16_offset(bytes, range.end);
  return NSMakeRange(start, end - start);
}

typedef struct {
  NSRect bounds;
  BOOL any, valid;
} TextBounds;

static void appendBounds(void *context, double x, double y, double width,
                         double height) {
  TextBounds *result = context;
  if (!isfinite(x) || !isfinite(y) || !isfinite(width) || !isfinite(height) ||
      !isfinite(x + width) || !isfinite(y + height) || width < 0 ||
      height < 0) {
    result->valid = NO;
    return;
  }
  NSRect next = NSMakeRect(x, y, width, height);
  if (!result->any)
    result->bounds = next;
  else {
    double left = fmin(NSMinX(result->bounds), x);
    double top = fmin(NSMinY(result->bounds), y);
    double right = fmax(NSMaxX(result->bounds), x + width);
    double bottom = fmax(NSMaxY(result->bounds), y + height);
    result->bounds = NSMakeRect(left, top, right - left, bottom - top);
  }
  result->any = YES;
}

@implementation SWAccessibilityNode (TextGeometry)
- (SWAccessibilityTextGeometry *)accessibleTextGeometry {
  SWAccessibilityState *state = self.state;
  return self.isAccessibilityElement && self.accessibleText &&
                 state.data.geometry
             ? state.textGeometry
             : nil;
}
- (NSRange)accessibilityRangeForIndex:(NSInteger)index {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  NSString *text = self.accessibleText;
  int64_t offset;
  if (!geometry || !textOffset(text, index, &offset))
    return unavailableRange();
  SWAccessibilityTextData value = geometry.value;
  return textRange(text, value.range(value.context, offset, 1, 0));
}
- (NSRange)accessibilityRangeForPosition:(NSPoint)point {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  SWView *view = self.tree.view;
  if (!geometry || !view.window || !isfinite(point.x) || !isfinite(point.y))
    return unavailableRange();
  NSPoint windowPoint = [view.window convertPointFromScreen:point];
  NSPoint local = [view convertPoint:windowPoint fromView:nil];
  SWAccessibilityTextData value = geometry.value;
  return textRange(self.accessibleText,
                   value.range_at_point(value.context, local.x, local.y));
}
- (NSRect)accessibilityFrameForRange:(NSRange)range {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  NSString *text = self.accessibleText;
  SWView *view = self.tree.view;
  if (!geometry || !view.window || !text || !sw_text_range(text, range))
    return NSZeroRect;
  int64_t start, end;
  if (!textOffset(text, (NSInteger)range.location, &start) ||
      !textOffset(text, (NSInteger)NSMaxRange(range), &end))
    return NSZeroRect;
  SWAccessibilityTextData value = geometry.value;
  TextBounds result = {.valid = YES};
  int32_t status =
      value.boxes(value.context, start, end, &result, appendBounds);
  return status == 1 && result.any && result.valid
             ? NSAccessibilityFrameInView(view, result.bounds)
             : NSZeroRect;
}
- (NSInteger)accessibilityLineForIndex:(NSInteger)index {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  int64_t offset;
  if (!geometry || !textOffset(self.accessibleText, index, &offset))
    return -1;
  SWAccessibilityTextData value = geometry.value;
  int64_t line = value.line_index(value.context, offset, 1);
  return line >= 0 ? (NSInteger)line : -1;
}
- (NSRange)accessibilityRangeForLine:(NSInteger)line {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  if (!geometry || line < 0)
    return unavailableRange();
  SWAccessibilityTextData value = geometry.value;
  return textRange(self.accessibleText, value.line_range(value.context, line));
}
- (NSInteger)accessibilityInsertionPointLineNumber {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  SWAccessibilityState *state = self.state;
  if (!geometry || state.selection.location == NSNotFound ||
      state.data.cursor < 0)
    return -1;
  SWAccessibilityTextData value = geometry.value;
  int64_t line =
      value.line_index(value.context, state.data.cursor,
                       (state.data.flags & SWAXCursorUpstream) ? 0 : 1);
  return line >= 0 ? (NSInteger)line : -1;
}
- (NSRange)accessibilityVisibleCharacterRange {
  SWAccessibilityTextGeometry *geometry = self.accessibleTextGeometry;
  if (!geometry)
    return unavailableRange();
  SWAccessibilityTextData value = geometry.value;
  return textRange(self.accessibleText, value.visible_range(value.context));
}
@end
