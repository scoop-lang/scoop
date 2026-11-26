#import "text.h"

static int64_t byteOffset(NSString *text, NSUInteger offset) {
  if (offset > text.length)
    return -1;
  return (int64_t)[[text substringToIndex:offset]
      lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
}
static BOOL boundary(NSString *text, NSUInteger offset) {
  return offset == 0 || offset == text.length ||
         !CFStringIsSurrogateLowCharacter([text characterAtIndex:offset]);
}
static SWITextEdit *edit(SWTextEditData data, NSString *text) {
  SWITextEdit *result = [SWITextEdit new];
  result.data = data;
  result.utf8 = [text dataUsingEncoding:NSUTF8StringEncoding];
  return result;
}

// Leave an identical prefix and suffix in place, without dividing a UTF-16
// surrogate pair.
static void plainChange(NSMutableArray *edits, NSString *before,
                        NSString *after) {
  NSUInteger start = 0, oldEnd = before.length, newEnd = after.length;
  while (start < oldEnd && start < newEnd &&
         [before characterAtIndex:start] == [after characterAtIndex:start])
    start++;
  if (!boundary(before, start) || !boundary(after, start))
    start--;
  while (oldEnd > start && newEnd > start &&
         [before characterAtIndex:oldEnd - 1] ==
             [after characterAtIndex:newEnd - 1]) {
    oldEnd--;
    newEnd--;
  }
  if (!boundary(before, oldEnd) || !boundary(after, newEnd)) {
    oldEnd++;
    newEnd++;
  }
  [edits addObject:edit(
                       (SWTextEditData){
                           .kind = 1,
                           .has_replacement = 1,
                           .replacement_start = byteOffset(before, start),
                           .replacement_end = byteOffset(before, oldEnd)},
                       [after substringWithRange:NSMakeRange(start,
                                                             newEnd - start)])];
}
static void preedit(NSMutableArray *edits, NSString *replaced, NSRange oldRange,
                    NSString *text, NSRange mark, NSRange selection) {
  NSString *composition = [text substringWithRange:mark];
  int64_t anchor = -1, cursor = -1;
  if (selection.location >= mark.location &&
      NSMaxRange(selection) <= NSMaxRange(mark)) {
    anchor = byteOffset(composition, selection.location - mark.location);
    cursor = byteOffset(composition, NSMaxRange(selection) - mark.location);
  }
  [edits addObject:edit((SWTextEditData){.kind = 0,
                                         .anchor = anchor,
                                         .cursor = cursor,
                                         .has_replacement = 1,
                                         .replacement_start = byteOffset(
                                             replaced, oldRange.location),
                                         .replacement_end = byteOffset(
                                             replaced, NSMaxRange(oldRange))},
                        composition)];
}
@implementation SWITextView (Changes)
- (void)publishChanges {
  NSString *before = self.projection ?: @"";
  NSString *after = self.text ?: @"";
  NSRange selection = self.selectedRange, mark = self.markedRange;
  BOOL textChanged = ![before isEqualToString:after];
  BOOL markChanged = !NSEqualRanges(mark, self.projectionMarked);
  BOOL selectionChanged = !NSEqualRanges(selection, self.projectionSelection);
  if (!textChanged && !markChanged && !selectionChanged)
    return;
  NSMutableArray *edits = [NSMutableArray array];
  if (mark.location != NSNotFound && (textChanged || markChanged)) {
    NSString *prefix = [after substringToIndex:mark.location];
    NSString *suffix = [after substringFromIndex:NSMaxRange(mark)];
    if (before.length >= prefix.length + suffix.length &&
        [before hasPrefix:prefix] && [before hasSuffix:suffix]) {
      preedit(edits, before,
              NSMakeRange(prefix.length,
                          before.length - prefix.length - suffix.length),
              after, mark, selection);
    } else {
      // An IME may commit part of a composition while starting a new marked
      // segment.
      if (textChanged)
        plainChange(edits, before, after);
      preedit(edits, after, mark, after, mark, selection);
    }
  } else {
    if (textChanged)
      plainChange(edits, before, after);
    else if (markChanged)
      [edits addObject:edit((SWTextEditData){.kind = 2}, nil)];
    if (selection.location != NSNotFound && (selectionChanged || textChanged))
      [edits
          addObject:edit(
                        (SWTextEditData){
                            .kind = 4,
                            .anchor = byteOffset(after, selection.location),
                            .cursor = byteOffset(after, NSMaxRange(selection))},
                        nil)];
  }
  [self rememberProjection];
  if (!edits.count)
    return;
  if (self.sequence == INT64_MAX)
    [NSException raise:NSInternalInconsistencyException
                format:@"Text edit sequence exhausted"];
  BOOL exactRevision = self.revision >= 0 && self.sequence == self.applied;
  self.sequence++;
  self.caretKnown = NO;
  SWIEvent *event = [SWIEvent new];
  event.header =
      (SWEventHeader){.window = self.content.owner.identity, .kind = 13};
  event.data = (SWEventData){.a = self.connection,
                             .b = self.revision,
                             .c = self.sequence,
                             .flags = exactRevision};
  event.edits = edits;
  [self.content.owner.application enqueue:event];
}
@end
