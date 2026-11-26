#import "text.h"

@implementation SWITextEdit
@end

@implementation SWITextView
- (instancetype)initWithContent:(SWIView *)content {
  self = [super initWithFrame:content.bounds textContainer:nil];
  if (self) {
    self.content = content;
    self.delegate = self;
    self.autoresizingMask =
        UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    // UIKit supplies UITextInput, its tokenizer and keyboard. Scoop draws the
    // editor.
    self.layer.opacity = 0;
    self.isAccessibilityElement = NO;
    self.accessibilityElementsHidden = YES;
    self.backgroundColor = UIColor.clearColor;
    self.scrollEnabled = NO;
    self.observedKeyboard = (SWSoftwareKeyboard){.state = 1};
    [content addSubview:self];
    content.keyboardLayoutGuide.followsUndockedKeyboard = YES;
    content.keyboardLayoutGuide.usesBottomSafeArea = NO;
    [self observeKeyboard];
  }
  return self;
}
- (UIView *)hitTest:(CGPoint)point withEvent:(UIEvent *)event {
  return nil;
}
- (NSRange)markedRange {
  UITextRange *marked = self.markedTextRange;
  if (!marked)
    return NSMakeRange(NSNotFound, 0);
  NSInteger start = [self offsetFromPosition:self.beginningOfDocument
                                  toPosition:marked.start];
  NSInteger end = [self offsetFromPosition:self.beginningOfDocument
                                toPosition:marked.end];
  return start >= 0 && end >= start ? NSMakeRange(start, end - start)
                                    : NSMakeRange(NSNotFound, 0);
}
- (void)rememberProjection {
  self.projection = self.text ?: @"";
  self.projectionSelection = self.selectedRange;
  self.projectionMarked = self.markedRange;
}
- (void)finishChange {
  if (--self.changeDepth == 0 && self.connected)
    [self publishChanges];
}
- (void)setMarkedText:(NSString *)text selectedRange:(NSRange)range {
  self.changeDepth++;
  @try {
    [super setMarkedText:text selectedRange:range];
  } @finally {
    [self finishChange];
  }
}
- (void)unmarkText {
  self.changeDepth++;
  @try {
    [super unmarkText];
  } @finally {
    [self finishChange];
  }
}
- (void)insertText:(NSString *)text {
  self.changeDepth++;
  @try {
    [super insertText:text];
  } @finally {
    [self finishChange];
  }
}
- (void)deleteBackward {
  self.changeDepth++;
  @try {
    [super deleteBackward];
  } @finally {
    [self finishChange];
  }
}
- (void)replaceRange:(UITextRange *)range withText:(NSString *)text {
  self.changeDepth++;
  @try {
    [super replaceRange:range withText:text];
  } @finally {
    [self finishChange];
  }
}
- (void)setSelectedTextRange:(UITextRange *)range {
  self.changeDepth++;
  @try {
    [super setSelectedTextRange:range];
  } @finally {
    [self finishChange];
  }
}
- (void)textViewDidChange:(UITextView *)view {
  if (self.connected && !self.changeDepth)
    [self publishChanges];
}
- (void)textViewDidChangeSelection:(UITextView *)view {
  if (self.connected && !self.changeDepth)
    [self publishChanges];
}
- (CGRect)caretRectForPosition:(UITextPosition *)position {
  return self.caretKnown ? [self convertRect:self.caret fromView:self.content]
                         : CGRectZero;
}
- (CGRect)firstRectForRange:(UITextRange *)range {
  return self.caretKnown ? [self convertRect:self.caret fromView:self.content]
                         : CGRectZero;
}
- (void)closeConnection {
  if (!self.connected)
    return;
  self.connected = NO;
  [self resignFirstResponder];
  [self unmarkText];
  self.publishedText = nil;
  self.caretKnown = NO;
}
@end

int32_t scoop_window_ime_allowed(void *pointer, int32_t allowed) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  SWITextView *editor = scene.view.editor;
  if (editor.connected == (allowed != 0))
    return 0;
  if (!editor) {
    editor = [[SWITextView alloc] initWithContent:scene.view];
    scene.view.editor = editor;
  }
  if (allowed) {
    if (editor.connection == INT64_MAX)
      return 1;
    editor.connection++;
    editor.sequence = editor.applied = 0;
    editor.revision = -1;
    editor.text = @"";
    [editor rememberProjection];
    editor.connected = YES;
  } else
    [editor closeConnection];
  SWIEvent *event = [SWIEvent new];
  event.header = (SWEventHeader){.window = scene.identity, .kind = 14};
  event.data = (SWEventData){.a = editor.connection, .flags = allowed != 0};
  [scene.application enqueue:event];
  return 0;
}

int64_t scoop_window_text_count(void *pointer) {
  return (int64_t)((__bridge SWIEvent *)pointer).edits.count;
}
SWTextEditData scoop_window_text_edit(void *pointer, int64_t index) {
  SWITextEdit *edit = ((__bridge SWIEvent *)pointer).edits[(NSUInteger)index];
  SWTextEditData data = edit.data;
  data.text = edit.utf8.bytes;
  data.text_length = (int64_t)edit.utf8.length;
  return data;
}
