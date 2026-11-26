#import "text.h"

typedef struct {
  int64_t connection, revision, after;
  const uint8_t *text;
  int64_t length, anchor, cursor, markedStart, markedEnd;
} SWITextSnapshot;

static NSUInteger utf16(const uint8_t *text, int64_t length) {
  if (length < 0)
    return NSNotFound;
  NSString *prefix = [[NSString alloc] initWithBytes:text
                                              length:(NSUInteger)length
                                            encoding:NSUTF8StringEncoding];
  return prefix ? prefix.length : NSNotFound;
}
static void applyPublished(SWITextView *editor, NSString *text,
                           NSRange selection, NSRange marked) {
  editor.changeDepth++;
  @try {
    BOOL sameText = [editor.text isEqualToString:text];
    if (!sameText || !NSEqualRanges(marked, editor.markedRange)) {
      if (editor.markedTextRange)
        [editor unmarkText];
      if (!sameText)
        editor.text = text;
      if (marked.location != NSNotFound) {
        editor.selectedRange = marked;
        NSRange relative = NSMakeRange(marked.length, 0);
        if (selection.location >= marked.location &&
            NSMaxRange(selection) <= NSMaxRange(marked))
          relative = NSMakeRange(selection.location - marked.location,
                                 selection.length);
        [editor setMarkedText:[text substringWithRange:marked]
                selectedRange:relative];
      }
    }
    if (!NSEqualRanges(editor.selectedRange, selection))
      editor.selectedRange = selection;
    [editor rememberProjection];
  } @finally {
    editor.changeDepth--;
  }
}
int64_t scoop_window_publish_text(void *pointer, SWITextSnapshot data) {
  SWITextView *editor = ((__bridge SWIScene *)pointer).view.editor;
  if (!editor.connected || editor.connection != data.connection)
    return 1;
  if (data.after < 0 || data.after > editor.sequence ||
      (data.after && data.after < editor.applied) ||
      data.revision < editor.revision)
    return 2;
  NSString *text = [[NSString alloc] initWithBytes:data.text
                                            length:(NSUInteger)data.length
                                          encoding:NSUTF8StringEncoding];
  NSUInteger anchor = utf16(data.text, data.anchor),
             cursor = utf16(data.text, data.cursor);
  if (!text || anchor == NSNotFound || cursor == NSNotFound)
    return 3;
  NSRange selection = NSMakeRange(MIN(anchor, cursor),
                                  MAX(anchor, cursor) - MIN(anchor, cursor));
  NSRange marked = NSMakeRange(NSNotFound, 0);
  if (data.markedStart >= 0) {
    NSUInteger start = utf16(data.text, data.markedStart),
               end = utf16(data.text, data.markedEnd);
    if (start == NSNotFound || end == NSNotFound || end < start)
      return 3;
    marked = NSMakeRange(start, end - start);
  }
  if (editor.revision == data.revision &&
      (![editor.publishedText isEqualToString:text] ||
       editor.publishedAnchor != data.anchor ||
       editor.publishedCursor != data.cursor ||
       !NSEqualRanges(editor.publishedMarked, marked)))
    return 2;
  if (editor.revision != data.revision)
    editor.caretKnown = NO;
  editor.revision = data.revision;
  editor.publishedText = text;
  editor.publishedAnchor = data.anchor;
  editor.publishedCursor = data.cursor;
  editor.publishedMarked = marked;
  if (data.after)
    editor.applied = data.after;
  if (editor.applied == editor.sequence)
    applyPublished(editor, text, selection, marked);
  return 0;
}
int64_t scoop_window_publish_text_caret(void *pointer, int64_t connection,
                                        int64_t revision, bool available,
                                        double x, double y, double width,
                                        double height) {
  SWITextView *editor = ((__bridge SWIScene *)pointer).view.editor;
  if (!editor.connected || editor.connection != connection ||
      editor.revision != revision || editor.sequence != editor.applied)
    return 1;
  editor.caret = CGRectMake(x, y, width, height);
  editor.caretKnown = available;
  return 0;
}
int64_t scoop_window_clear_ime_caret(void *pointer) {
  ((__bridge SWIScene *)pointer).view.editor.caretKnown = NO;
  return 0;
}
int64_t scoop_window_ime_caret(void *pointer, double x, double y, double width,
                               double height) {
  SWITextView *editor = ((__bridge SWIScene *)pointer).view.editor;
  if (editor.connected) {
    editor.caret = CGRectMake(x, y, width, height);
    editor.caretKnown = YES;
  }
  return 0;
}
