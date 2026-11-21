#import "../../../scoop.window/native/darwin/accessibility.h"
#import <AppKit/AppKit.h>
#include <stdint.h>

int64_t scoop_ui_ax_trait(void *pointer, int32_t field) {
  id node = (__bridge id)pointer;
  switch (field) {
  case 0:
    return [node isAccessibilitySelected];
  case 1:
    return [node isAccessibilityExpanded];
  case 2:
    return [[node accessibilityMinValue] longLongValue];
  case 3:
    return [[node accessibilityMaxValue] longLongValue];
  case 4:
    return (int64_t)[[node accessibilitySelectedChildren] count];
  case 5:
    return [node accessibilityRowCount];
  case 6:
    return [node accessibilityColumnCount];
  case 7:
    return (int64_t)[[node accessibilityRows] count];
  case 8:
    return
        [node isAccessibilitySelectorAllowed:@selector(setAccessibilityValue:)];
  case 9:
    return [node isAccessibilityModal];
  case 10:
    return (int64_t)[[node accessibilityDisclosedRows] count];
  default:
    return -1;
  }
}
int64_t scoop_ui_ax_role_count(void *pointer, const uint8_t *bytes,
                               int64_t length) {
  NSString *role = [[NSString alloc] initWithBytes:bytes
                                            length:(NSUInteger)length
                                          encoding:NSUTF8StringEncoding];
  NSMutableArray *pending =
      [NSMutableArray arrayWithObject:(__bridge id)pointer];
  int64_t result = 0;
  while (pending.count) {
    id node = pending.lastObject;
    [pending removeLastObject];
    if ([[node accessibilityRole] isEqualToString:role])
      ++result;
    [pending addObjectsFromArray:[node accessibilityChildren] ?: @[]];
  }
  return result;
}
int32_t scoop_ui_ax_table_span(void *pointer) {
  id table = (__bridge id)pointer;
  id first = [table accessibilityCellForColumn:0 row:1];
  id second = [table accessibilityCellForColumn:1 row:1];
  if (!first || first != second)
    return 0;
  return NSEqualRanges([first accessibilityColumnIndexRange],
                       NSMakeRange(0, 2)) &&
         [[first accessibilityColumnHeaderUIElements] count] == 2;
}

@interface SWUiRetainedTextRange : NSObject
@property(nonatomic, strong) SWAccessibilityNode *node;
@property(nonatomic) SWTextRange *range;
@property(nonatomic) SWTextDocument *document;
@property(nonatomic) uint64_t epoch;
@end
@implementation SWUiRetainedTextRange
- (void)dealloc {
  sw_text_range_release(_range);
}
@end
void *scoop_ui_ax_retain_text_range(void *pointer, int64_t start, int64_t end) {
  SWAccessibilityNode *node = (__bridge SWAccessibilityNode *)pointer;
  SWAccessibilityState *state = node.state;
  if (!state.textDocument)
    return NULL;
  SWTextRange *range = sw_text_range_create(
      state.textDocument, state.documentRevision, start, end, 1, 0);
  if (!range)
    return NULL;
  SWUiRetainedTextRange *retained = [SWUiRetainedTextRange new];
  retained.node = node;
  retained.range = range;
  retained.document = state.textDocument;
  retained.epoch = state.documentEpoch;
  return (__bridge_retained void *)retained;
}
int32_t scoop_ui_ax_retained_range(void *pointer, int64_t start, int64_t end) {
  SWUiRetainedTextRange *range = (__bridge SWUiRetainedTextRange *)pointer;
  SWAccessibilityState *state = range.node.state;
  BOOL current = state && state.textDocument == range.document &&
                 state.documentEpoch == range.epoch;
  SWTextRangeState value =
      current ? sw_text_range_read(range.range, state.documentRevision)
              : (SWTextRangeState){0};
  return start < 0 ? !value.status
                   : value.status && value.start == start && value.end == end;
}
void scoop_ui_ax_release_range(void *pointer) {
  if (pointer)
    (void)CFBridgingRelease(pointer);
}

void *scoop_ui_ax_disclosure(void *pointer, int64_t index) {
  id node = (__bridge id)pointer;
  if (index < 0)
    return (__bridge_retained void *)[node accessibilityDisclosedByRow];
  NSArray *rows = [node accessibilityDisclosedRows];
  return (uint64_t)index < rows.count
             ? (__bridge_retained void *)rows[(NSUInteger)index]
             : NULL;
}
