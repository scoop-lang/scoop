#import "accessibility.h"
#include <string.h>

@implementation SWAccessibilityNode (Text)
- (NSString *)accessibleText {
  SWAccessibilityState *state = self.state;
  if (!state || (state.data.flags & SWAXProtected) ||
      (state.data.role != 1 && state.data.role != 4))
    return nil;
  return state.value;
}
- (NSInteger)accessibilityNumberOfCharacters {
  return (NSInteger)self.accessibleText.length;
}
- (NSRange)accessibilitySelectedTextRange {
  SWAccessibilityState *state = self.state;
  return self.accessibleText ? state.selection : NSMakeRange(NSNotFound, 0);
}
- (NSArray<NSValue *> *)accessibilitySelectedTextRanges {
  NSRange range = self.accessibilitySelectedTextRange;
  return range.location == NSNotFound ? @[]
                                      : @[ [NSValue valueWithRange:range] ];
}
- (NSString *)accessibilitySelectedText {
  return [self accessibilityStringForRange:self.accessibilitySelectedTextRange];
}
- (NSString *)accessibilityStringForRange:(NSRange)range {
  NSString *text = self.accessibleText;
  return text && sw_text_range(text, range) ? [text substringWithRange:range]
                                            : nil;
}
- (NSAttributedString *)accessibilityAttributedStringForRange:(NSRange)range {
  NSString *text = [self accessibilityStringForRange:range];
  return text ? [[NSAttributedString alloc] initWithString:text] : nil;
}
- (void)setAccessibilitySelectedTextRange:(NSRange)range {
  if (self.state.data.actions & 16)
    [self perform:5 text:nil range:range];
}
- (void)setAccessibilitySelectedTextRanges:(NSArray<NSValue *> *)ranges {
  if (![ranges isKindOfClass:NSArray.class] || ranges.count != 1)
    return;
  NSValue *value = ranges.firstObject;
  if (![value isKindOfClass:NSValue.class] ||
      strcmp(value.objCType, @encode(NSRange)))
    return;
  [self setAccessibilitySelectedTextRange:value.rangeValue];
}
- (void)setAccessibilitySelectedText:(NSString *)text {
  SWAccessibilityState *state = self.state;
  if (!state || !(state.data.actions & 32) ||
      (state.data.flags & SWAXReadOnly) || ![text isKindOfClass:NSString.class])
    return;
  [self perform:6 text:text range:state.selection];
}
@end
