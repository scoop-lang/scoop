#import "window.h"
#include <stdbool.h>

@implementation SWTextSnapshot
@end

typedef struct {
  int64_t connection, revision, after;
  const uint8_t *text;
  int64_t length, anchor, cursor, marked_start, marked_end;
} SWTextSnapshotData;

static NSRange selected(SWTextSnapshot *value) {
  NSUInteger start = MIN(value.anchor, value.cursor);
  return NSMakeRange(start, MAX(value.anchor, value.cursor) - start);
}

SWTextSnapshot *sw_copy_text_snapshot(SWTextSnapshot *value) {
  if (!value)
    return nil;
  SWTextSnapshot *copy = [SWTextSnapshot new];
  copy.text = value.text;
  copy.anchor = value.anchor;
  copy.cursor = value.cursor;
  copy.marked = value.marked;
  copy.revision = value.revision;
  return copy;
}

SWTextSnapshot *sw_query_text_snapshot(SWView *view) {
  if (view.textBatchDepth)
    return view.textProjection;
  return view.appliedTextSequence == view.textSequence ? view.textSnapshot
                                                       : nil;
}

void sw_invalidate_text_caret(SWView *view) {
  view.imeCaretKnown = NO;
  if (view.imeAllowed)
    [view.inputContext invalidateCharacterCoordinates];
}

static NSUInteger utf16_offset(const uint8_t *text, int64_t length) {
  if (length < 0)
    return NSNotFound;
  NSString *prefix = [[NSString alloc] initWithBytes:text
                                              length:(NSUInteger)length
                                            encoding:NSUTF8StringEncoding];
  return prefix ? prefix.length : NSNotFound;
}

static void synchronize_composition(SWView *view, SWTextSnapshot *value) {
  view.documentCoordinatesKnown = YES;
  view.documentSelection = selected(value);
  view.documentMarked = value.marked;
  view.markedText = value.marked.location == NSNotFound
                        ? @""
                        : [value.text substringWithRange:value.marked];
  NSRange selection = view.documentSelection;
  if (value.marked.location != NSNotFound &&
      selection.location >= value.marked.location &&
      NSMaxRange(selection) <= NSMaxRange(value.marked))
    view.markedSelection = NSMakeRange(
        selection.location - value.marked.location, selection.length);
  else
    view.markedSelection = NSMakeRange(NSNotFound, 0);
}

int64_t scoop_window_publish_text(void *pointer, SWTextSnapshotData data) {
  @autoreleasepool {
    @try {
      SWView *view = ((__bridge SWWindow *)pointer).view;
      if (!view.imeAllowed || view.textConnection != data.connection)
        return 1;
      if (data.after < 0 || data.after > view.textSequence ||
          (data.after && data.after < view.appliedTextSequence) ||
          (view.textSnapshot && data.revision < view.textSnapshot.revision))
        return 2;
      SWTextSnapshot *next = [SWTextSnapshot new];
      next.text = [[NSString alloc] initWithBytes:data.text
                                           length:(NSUInteger)data.length
                                         encoding:NSUTF8StringEncoding];
      next.anchor = utf16_offset(data.text, data.anchor);
      next.cursor = utf16_offset(data.text, data.cursor);
      NSUInteger start = utf16_offset(data.text, data.marked_start);
      NSUInteger end = utf16_offset(data.text, data.marked_end);
      next.marked = start == NSNotFound ? NSMakeRange(NSNotFound, 0)
                                        : NSMakeRange(start, end - start);
      next.revision = data.revision;
      if (!next.text || next.anchor == NSNotFound || next.cursor == NSNotFound)
        return 3;
      SWTextSnapshot *previous = view.textSnapshot;
      if (previous && previous.revision == next.revision &&
          (![previous.text isEqualToString:next.text] ||
           previous.anchor != next.anchor || previous.cursor != next.cursor ||
           !NSEqualRanges(previous.marked, next.marked)))
        return 2;
      view.textSnapshot = next;
      if (data.after)
        view.appliedTextSequence = data.after;
      if (!previous || previous.revision != next.revision)
        sw_invalidate_text_caret(view);
      if (view.appliedTextSequence == view.textSequence)
        synchronize_composition(view, next);
      return 0;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

int64_t scoop_window_publish_text_caret(void *pointer, int64_t connection,
                                        int64_t revision, bool available,
                                        double x, double y, double width,
                                        double height) {
  @autoreleasepool {
    @try {
      SWView *view = ((__bridge SWWindow *)pointer).view;
      if (!view.imeAllowed || view.textConnection != connection ||
          !view.textSnapshot || view.textSnapshot.revision != revision ||
          view.appliedTextSequence != view.textSequence)
        return 1;
      view.imeCaret = NSMakeRect(x, y, width, height);
      view.imeCaretRevision = revision;
      view.imeCaretKnown = available;
      [view.inputContext invalidateCharacterCoordinates];
      return 0;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

int64_t scoop_window_clear_ime_caret(void *pointer) {
  @autoreleasepool {
    @try {
      sw_invalidate_text_caret(((__bridge SWWindow *)pointer).view);
      return 0;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}
