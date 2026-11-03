#import "window.h"

static NSRange selected(SWTextSnapshot *value) {
  NSUInteger start = MIN(value.anchor, value.cursor);
  return NSMakeRange(start, MAX(value.anchor, value.cursor) - start);
}

BOOL sw_document_edit(SWView *view, int32_t kind, NSString *text,
                      NSRange selection, NSRange replacement) {
  SWTextSnapshot *projection = view.textProjection;
  BOOL explicit_range = replacement.location != NSNotFound;
  if (explicit_range &&
      (!projection || !sw_text_range(projection.text, replacement)))
    return NO;
  if (kind == 0 && !text.length && !view.markedText.length)
    return YES;

  NSRange range = replacement;
  if (!explicit_range) {
    if (projection)
      range = projection.marked.location != NSNotFound ? projection.marked
                                                       : selected(projection);
    else if (view.documentCoordinatesKnown)
      range = view.documentMarked.location != NSNotFound
                  ? view.documentMarked
                  : view.documentSelection;
  }
  if (range.location != NSNotFound &&
      text.length > NSUIntegerMax - range.location)
    return NO;

  sw_append_text(view, kind, text, selection);
  if (view.textBatchInvalid)
    return NO;
  if (explicit_range) {
    SWTextEdit *edit = view.textEdits.lastObject;
    SWTextEditData data = edit.data;
    data.has_replacement = 1;
    data.replacement_start =
        (int64_t)[[projection.text substringToIndex:range.location]
            lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
    data.replacement_end =
        (int64_t)[[projection.text substringToIndex:NSMaxRange(range)]
            lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
    edit.data = data;
    view.textBatchUsesRange = YES;
  }

  BOOL marked = kind == 0 && text.length;
  NSRange next_selection = NSMakeRange(NSNotFound, 0);
  NSRange next_marked = NSMakeRange(NSNotFound, 0);
  if (range.location != NSNotFound) {
    next_selection =
        marked && selection.location != NSNotFound
            ? NSMakeRange(range.location + selection.location, selection.length)
            : NSMakeRange(range.location + text.length, 0);
    if (marked)
      next_marked = NSMakeRange(range.location, text.length);
  }
  if (projection) {
    projection.text = [projection.text stringByReplacingCharactersInRange:range
                                                               withString:text];
    projection.anchor = next_selection.location;
    projection.cursor = NSMaxRange(next_selection);
    projection.marked = next_marked;
  }
  view.documentCoordinatesKnown = next_selection.location != NSNotFound;
  view.documentSelection = next_selection;
  view.documentMarked = next_marked;
  view.markedText = marked ? text : @"";
  view.markedSelection = marked ? selection : NSMakeRange(NSNotFound, 0);
  return YES;
}
