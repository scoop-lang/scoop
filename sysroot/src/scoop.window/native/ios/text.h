#pragma once
#import "../software_keyboard.h"
#import "window.h"

@interface SWITextEdit : NSObject
@property(nonatomic) SWTextEditData data;
@property(nonatomic, strong) NSData *utf8;
@end

@interface SWITextView : UITextView <UITextViewDelegate>
@property(nonatomic, weak) SWIView *content;
@property(nonatomic) BOOL connected;
@property(nonatomic) int64_t connection, sequence, applied, revision;
@property(nonatomic, copy) NSString *publishedText, *projection;
@property(nonatomic) NSRange publishedMarked, projectionSelection,
    projectionMarked;
@property(nonatomic) int64_t publishedAnchor, publishedCursor;
@property(nonatomic) NSUInteger changeDepth;
@property(nonatomic) BOOL caretKnown, keyboardVisible;
@property(nonatomic) CGRect caret;
@property(nonatomic) SWSoftwareKeyboard observedKeyboard;
- (instancetype)initWithContent:(SWIView *)content;
- (void)closeConnection;
- (NSRange)markedRange;
- (void)rememberProjection;
@end

@interface SWITextView (Changes)
- (void)publishChanges;
@end

@interface SWITextView (Keyboard)
- (void)observeKeyboard;
- (void)updateKeyboardGeometry;
@end
