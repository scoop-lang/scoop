#import "window.h"

@implementation SWTextEdit
@end

BOOL sw_text_boundary(NSString *text, NSUInteger offset) {
  if (offset == 0 || offset == text.length)
    return YES;
  unichar previous = [text characterAtIndex:offset - 1];
  unichar next = [text characterAtIndex:offset];
  return !(CFStringIsSurrogateHighCharacter(previous) &&
           CFStringIsSurrogateLowCharacter(next));
}

BOOL sw_text_range(NSString *text, NSRange range) {
  return range.location <= text.length &&
         range.length <= text.length - range.location &&
         sw_text_boundary(text, range.location) &&
         sw_text_boundary(text, NSMaxRange(range));
}

static NSString *plain_text(id value) {
  if ([value isKindOfClass:NSAttributedString.class])
    return [[value string] copy];
  if ([value isKindOfClass:NSString.class])
    return [value copy];
  return nil;
}

static BOOL printable(NSString *text) {
  if (!text.length)
    return NO;
  unichar first = [text characterAtIndex:0];
  return first >= 0x20 && first != 0x7f &&
         !(first >= 0xf700 && first <= 0xf8ff);
}

void sw_append_text(SWView *view, int32_t kind, NSString *text,
                    NSRange selection) {
  NSData *bytes = [text dataUsingEncoding:NSUTF8StringEncoding
                     allowLossyConversion:NO];
  if (!bytes) {
    view.textBatchInvalid = YES;
    return;
  }
  SWTextEdit *edit = [SWTextEdit new];
  SWTextEditData data = {0};
  data.kind = kind;
  data.anchor = data.cursor = -1;
  if (selection.location != NSNotFound) {
    data.anchor = (int64_t)[[text substringToIndex:selection.location]
        lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
    data.cursor = (int64_t)[[text substringToIndex:NSMaxRange(selection)]
        lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
  }
  edit.data = data;
  edit.text = bytes;
  [view.textEdits addObject:edit];
}

static void discard_marked(SWView *view) {
  if (view.discardingText || !view.imeAllowed)
    return;
  view.discardingText = YES;
  @try {
    [view.inputContext discardMarkedText];
  } @finally {
    view.discardingText = NO;
  }
}

// Explicit replacement ranges can only refer to the native marked segment.
static NSString *replace_marked(SWView *view, NSString *text,
                                NSRange replacement, NSUInteger *offset) {
  NSString *previous = view.markedText ?: @"";
  if (replacement.location == NSNotFound)
    replacement = NSMakeRange(0, previous.length);
  if (!sw_text_range(previous, replacement))
    return nil;
  *offset = replacement.location;
  return [previous stringByReplacingCharactersInRange:replacement
                                           withString:text];
}

void sw_commit(SWView *view, NSString *text) {
  if (!printable(text))
    return;
  [view beginTextBatch];
  sw_append_text(view, 1, text, NSMakeRange(NSNotFound, 0));
  [view endTextBatch];
}

@implementation SWView (Text)
- (void)beginTextBatch {
  if (self.textBatchDepth++ != 0)
    return;
  self.textEdits = [NSMutableArray new];
  self.textCommands = [NSMutableArray new];
  self.textProjection = sw_copy_text_snapshot(
      self.appliedTextSequence == self.textSequence ? self.textSnapshot : nil);
  self.textBatchUsesRange = NO;
  self.textBatchHadMarked = self.markedText.length != 0;
  self.textBatchInvalid = NO;
  self.compositionConsumed = self.textBatchHadMarked;
  self.compositionCommitted = NO;
  self.forwardKey = NO;
}
- (void)endTextBatch {
  if (--self.textBatchDepth != 0)
    return;
  if (self.textBatchInvalid) {
    [self.textEdits removeAllObjects];
    self.markedText = @"";
    self.markedSelection = NSMakeRange(NSNotFound, 0);
    self.documentCoordinatesKnown = NO;
    self.textBatchUsesRange = NO;
    discard_marked(self);
    // Only remove a preedit the application has already received.
    if (self.textBatchHadMarked)
      sw_append_text(self, 0, @"", NSMakeRange(NSNotFound, 0));
  }
  if (self.textEdits.count) {
    SWEvent *event = sw_event(self.owner, SWTextBatch);
    event.edits = [self.textEdits copy];
    SWEventData data = event.data;
    if (self.imeAllowed) {
      if (self.textSequence == INT64_MAX)
        [NSException raise:NSRangeException
                    format:@"native text batch sequence overflow"];
      data.a = self.textConnection;
      data.c = ++self.textSequence;
    }
    if (self.textBatchUsesRange) {
      data.flags = 1;
      data.b = self.textSnapshot.revision;
    }
    event.data = data;
    if (self.textSnapshot)
      sw_invalidate_text_caret(self);
    [self.owner.application enqueue:event];
  }
  self.textEdits = nil;
  self.textCommands = nil;
  self.textProjection = nil;
}
- (void)cancelComposition {
  if (self.discardingText)
    return;
  [self beginTextBatch];
  if (self.markedText.length) {
    if (self.textSnapshot) {
      if (!sw_document_edit(self, 0, @"", NSMakeRange(NSNotFound, 0),
                            NSMakeRange(NSNotFound, 0)))
        self.textBatchInvalid = YES;
    } else
      sw_append_text(self, 0, @"", NSMakeRange(NSNotFound, 0));
    self.compositionConsumed = YES;
  }
  self.markedText = @"";
  self.markedSelection = NSMakeRange(NSNotFound, 0);
  discard_marked(self);
  [self endTextBatch];
}
- (BOOL)hasMarkedText {
  return self.markedText.length != 0;
}
- (NSRange)markedRange {
  if (self.textSnapshot)
    return self.documentCoordinatesKnown ? self.documentMarked
                                         : NSMakeRange(NSNotFound, 0);
  return self.markedText.length ? NSMakeRange(0, self.markedText.length)
                                : NSMakeRange(NSNotFound, 0);
}
- (NSRange)selectedRange {
  if (self.textSnapshot)
    return self.documentCoordinatesKnown ? self.documentSelection
                                         : NSMakeRange(NSNotFound, 0);
  return self.markedText.length ? self.markedSelection
                                : NSMakeRange(NSNotFound, 0);
}
- (void)setMarkedText:(id)value
        selectedRange:(NSRange)selection
     replacementRange:(NSRange)replacement {
  if (!self.imeAllowed || self.discardingText)
    return;
  [self beginTextBatch];
  self.compositionConsumed = YES;
  NSString *text = plain_text(value);
  if (self.textSnapshot) {
    if (!text ||
        (selection.location != NSNotFound && !sw_text_range(text, selection)) ||
        !sw_document_edit(self, 0, text, selection, replacement))
      self.textBatchInvalid = YES;
    [self endTextBatch];
    return;
  }
  NSUInteger offset = 0;
  NSString *updated =
      text ? replace_marked(self, text, replacement, &offset) : nil;
  if (!updated ||
      (selection.location != NSNotFound && !sw_text_range(text, selection))) {
    self.textBatchInvalid = YES;
  } else {
    NSRange shifted =
        selection.location == NSNotFound || !updated.length
            ? NSMakeRange(NSNotFound, 0)
            : NSMakeRange(offset + selection.location, selection.length);
    if (updated.length || self.markedText.length)
      sw_append_text(self, 0, updated, shifted);
    self.markedText = updated;
    self.markedSelection = shifted;
  }
  [self endTextBatch];
}
- (void)insertText:(id)value replacementRange:(NSRange)replacement {
  if (!self.imeAllowed || self.discardingText)
    return;
  [self beginTextBatch];
  NSString *text = plain_text(value);
  BOOL hadMarked = self.markedText.length != 0;
  if (!text) {
    self.textBatchInvalid = YES;
  } else if (self.textSnapshot) {
    if (!sw_document_edit(self, 1, text, NSMakeRange(NSNotFound, 0),
                          replacement))
      self.textBatchInvalid = YES;
    else if (hadMarked) {
      self.compositionConsumed = YES;
      self.compositionCommitted = YES;
    }
  } else {
    NSUInteger offset = 0;
    NSString *updated = replace_marked(self, text, replacement, &offset);
    if (!updated) {
      self.textBatchInvalid = YES;
    } else {
      if (self.markedText.length) {
        self.compositionConsumed = YES;
        self.compositionCommitted = YES;
      }
      sw_append_text(self, 1, updated, NSMakeRange(NSNotFound, 0));
      self.markedText = @"";
      self.markedSelection = NSMakeRange(NSNotFound, 0);
    }
  }
  [self endTextBatch];
}
- (void)unmarkText {
  if (!self.imeAllowed || self.discardingText || !self.markedText.length)
    return;
  [self beginTextBatch];
  sw_append_text(self, 2, @"", NSMakeRange(NSNotFound, 0));
  self.markedText = @"";
  self.markedSelection = NSMakeRange(NSNotFound, 0);
  self.documentMarked = NSMakeRange(NSNotFound, 0);
  self.textProjection.marked = NSMakeRange(NSNotFound, 0);
  self.compositionConsumed = YES;
  self.compositionCommitted = YES;
  [self endTextBatch];
}
- (void)doCommandBySelector:(SEL)command {
  // Commands after an IME edit belong to the already-consumed native key.
  if (!self.compositionCommitted &&
      !(self.compositionConsumed && self.textEdits.count)) {
    self.forwardKey = YES;
    int32_t code = sw_text_command(command);
    if (code)
      [self.textCommands addObject:@(code)];
  }
}
- (NSArray<NSAttributedStringKey> *)validAttributesForMarkedText {
  return @[];
}
- (NSAttributedString *)attributedSubstringForProposedRange:(NSRange)range
                                                actualRange:
                                                    (NSRangePointer)actual {
  if (actual)
    *actual = NSMakeRange(NSNotFound, 0);
  SWTextSnapshot *snapshot = sw_query_text_snapshot(self);
  if (!snapshot || !sw_text_range(snapshot.text, range))
    return nil;
  if (actual)
    *actual = range;
  return [[NSAttributedString alloc]
      initWithString:[snapshot.text substringWithRange:range]];
}
- (NSUInteger)characterIndexForPoint:(NSPoint)point {
  SWTextSnapshot *snapshot = sw_query_text_snapshot(self);
  if (snapshot && self.imeCaretKnown &&
      self.imeCaretRevision == snapshot.revision) {
    NSRect rect = [self.window
        convertRectToScreen:[self convertRect:self.imeCaret toView:nil]];
    if (NSPointInRect(point, rect))
      return snapshot.cursor;
  }
  return NSNotFound;
}
- (NSRect)firstRectForCharacterRange:(NSRange)range
                         actualRange:(NSRangePointer)actual {
  if (actual)
    *actual = NSMakeRange(NSNotFound, 0);
  if (!self.imeCaretKnown)
    return NSZeroRect;
  if (self.textSnapshot) {
    SWTextSnapshot *snapshot = sw_query_text_snapshot(self);
    if (!snapshot || self.imeCaretRevision != snapshot.revision ||
        (range.location != NSNotFound &&
         (!sw_text_range(snapshot.text, range) ||
          snapshot.cursor < range.location ||
          snapshot.cursor > NSMaxRange(range))))
      return NSZeroRect;
    if (actual)
      *actual = NSMakeRange(snapshot.cursor, 0);
  }
  return [self.window convertRectToScreen:[self convertRect:self.imeCaret
                                                     toView:nil]];
}
@end

int32_t scoop_window_ime_allowed(void *pointer, int32_t allowed) {
  SWWindow *window = (__bridge SWWindow *)pointer;
  SWView *view = window.view;
  if (view.imeAllowed == (allowed != 0))
    return 0;
  NSTextInputContext *context = view.inputContext;
  if (allowed && !context)
    return 1;
  if (allowed && view.textConnection == INT64_MAX)
    return 1;
  if (!allowed)
    [view cancelComposition];
  if (allowed) {
    ++view.textConnection;
    view.textSequence = 0;
    view.appliedTextSequence = 0;
  }
  view.textSnapshot = nil;
  view.textProjection = nil;
  view.documentCoordinatesKnown = NO;
  if (!allowed)
    sw_invalidate_text_caret(view);
  view.imeAllowed = allowed != 0;
  SWEvent *event = sw_event(window, SWImeState);
  SWEventData data = event.data;
  data.flags = allowed != 0;
  data.a = view.textConnection;
  event.data = data;
  [window.application enqueue:event];
  if (window.window.keyWindow) {
    if (allowed)
      [context activate];
    else
      [context deactivate];
  }
  return 0;
}

int64_t scoop_window_ime_caret(void *pointer, double x, double y, double width,
                               double height) {
  SWView *view = ((__bridge SWWindow *)pointer).view;
  view.imeCaret = NSMakeRect(x, y, width, height);
  view.imeCaretKnown = YES;
  view.imeCaretRevision = -1;
  if (view.imeAllowed)
    [view.inputContext invalidateCharacterCoordinates];
  return 0;
}

int64_t scoop_window_text_count(void *pointer) {
  return (int64_t)((__bridge SWEvent *)pointer).edits.count;
}

SWTextEditData scoop_window_text_edit(void *pointer, int64_t index) {
  SWTextEdit *edit = ((__bridge SWEvent *)pointer).edits[(NSUInteger)index];
  SWTextEditData data = edit.data;
  data.text = edit.text.bytes;
  data.text_length = (int64_t)edit.text.length;
  return data;
}
