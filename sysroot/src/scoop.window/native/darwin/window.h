#ifndef SCOOP_WINDOW_DARWIN_H
#define SCOOP_WINDOW_DARWIN_H

#include "../frame.h"
#import <AppKit/AppKit.h>
#import <CoreFoundation/CoreFoundation.h>
#include <stdint.h>

typedef int32_t (*SWCallback)(void *, int64_t, int32_t);
typedef struct {
  int64_t window;
  int32_t kind, flags;
  int64_t a, b, c, d;
  double x, y, z, w, scale, dx, dy;
  const uint8_t *text;
  int64_t text_length;
  int64_t time_seconds, time_nanoseconds;
} SWEventData;
typedef struct {
  int32_t kind;
  int64_t anchor, cursor;
  const uint8_t *text;
  int64_t text_length;
  int32_t has_replacement;
  int64_t replacement_start, replacement_end;
} SWTextEditData;
typedef struct {
  double width, height, pixel_width, pixel_height, scale;
  double x, y, outer_width, outer_height;
  int32_t visible, occluded, focused, minimized, maximized, dark;
} SWSnapshot;

typedef struct {
  void *pointer;
  int64_t error;
} SWResult;

@class SWApplication, SWWindow, SWMenu;

@interface SWTextSnapshot : NSObject
@property(nonatomic, strong) NSString *text;
@property(nonatomic) NSUInteger anchor, cursor;
@property(nonatomic) NSRange marked;
@property(nonatomic) int64_t revision;
@end

@interface SWTextEdit : NSObject
@property(nonatomic) SWTextEditData data;
@property(nonatomic, strong) NSData *text;
@end

@interface SWEvent : NSObject
@property(nonatomic) SWEventData data;
@property(nonatomic, strong) NSEvent *nativeInput;
@property(nonatomic, strong) NSData *text;
@property(nonatomic, strong) NSArray<SWTextEdit *> *edits;
@property(nonatomic, strong) NSArray<NSNumber *> *commands;
@end

@interface SWApplication : NSObject <NSApplicationDelegate>
@property(nonatomic) SWCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) CFRunLoopSourceRef source;
@property(nonatomic) CFRunLoopTimerRef timer;
@property(nonatomic) BOOL deadlineFired;
@property(nonatomic, strong) NSMutableArray<SWEvent *> *events;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWWindow *> *windows;
@property(nonatomic, strong) NSMutableDictionary<NSNumber *, id> *watches;
@property(nonatomic, weak) SWMenu *menuBar;
- (void)enqueue:(SWEvent *)event;
@end

@interface SWView : NSView
@property(nonatomic, weak) SWWindow *owner;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWEvent *> *pressedKeys;
@property(nonatomic, strong) NSMutableSet<NSNumber *> *pressedButtons;
@property(nonatomic) NSEventModifierFlags modifiers;
@property(nonatomic) BOOL functionHeld;
@property(nonatomic, strong) NSTrackingArea *tracking;
@property(nonatomic, strong) NSCursor *cursor;
@property(nonatomic) BOOL pointerInside, cursorVisible, cursorHidden;
@property(nonatomic) BOOL imeAllowed, discardingText;
@property(nonatomic) NSRect imeCaret;
@property(nonatomic) BOOL imeCaretKnown;
@property(nonatomic) int64_t imeCaretRevision, textConnection, textSequence,
    appliedTextSequence;
@property(nonatomic, strong) SWTextSnapshot *textSnapshot, *textProjection;
@property(nonatomic) BOOL textBatchUsesRange, documentCoordinatesKnown;
@property(nonatomic) NSRange documentSelection, documentMarked;
@property(nonatomic, strong) NSString *markedText;
@property(nonatomic) NSRange markedSelection;
@property(nonatomic, strong) NSMutableArray<SWTextEdit *> *textEdits;
@property(nonatomic, strong) NSMutableArray<NSNumber *> *textCommands;
@property(nonatomic) NSUInteger textBatchDepth;
@property(nonatomic) BOOL textBatchHadMarked, textBatchInvalid;
@property(nonatomic) BOOL compositionConsumed, compositionCommitted, forwardKey;
@end

@interface SWView (Input)
- (void)resetInput;
@end
NSEventModifierFlags sw_input_modifiers(SWView *view,
                                        NSEventModifierFlags flags);
@interface SWView (Pointer)
- (void)updateCursor;
- (void)cancelPointer:(int32_t)reason;
@end
@interface SWView (Text) <NSTextInputClient>
- (void)beginTextBatch;
- (void)endTextBatch;
- (void)cancelComposition;
@end

enum {
  SWKeyboard = 10,
  SWModifiers = 11,
  SWPointer = 12,
  SWTextBatch = 13,
  SWImeState = 14
};
void sw_commit(SWView *view, NSString *text);
void sw_modifiers(SWView *view, NSEventModifierFlags flags);
int32_t sw_text_command(SEL selector);
BOOL sw_text_boundary(NSString *text, NSUInteger offset);
BOOL sw_text_range(NSString *text, NSRange range);
void sw_append_text(SWView *view, int32_t kind, NSString *text,
                    NSRange selection);
BOOL sw_document_edit(SWView *view, int32_t kind, NSString *text,
                      NSRange selection, NSRange replacement);
SWTextSnapshot *sw_copy_text_snapshot(SWTextSnapshot *value);
SWTextSnapshot *sw_query_text_snapshot(SWView *view);
void sw_invalidate_text_caret(SWView *view);

@interface SWWindow : NSObject <NSWindowDelegate>
@property(nonatomic, weak) SWApplication *application;
@property(nonatomic) int64_t identity;
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic, strong) SWView *view;
@property(nonatomic, strong) NSVisualEffectView *effect;
@property(nonatomic) BOOL transparent;
@property(nonatomic, strong) NSData *hitRegions;
@property(nonatomic) double frameWidth, frameHeight, frameScale;
@property(nonatomic) NSInteger attentionRequest;
- (void)emit:(int32_t)kind;
@end

void sw_wake(SWApplication *application);
BOOL sw_menu_key_equivalent(SWApplication *application, NSEvent *event);
SWEvent *sw_event(SWWindow *window, int32_t kind);
void sw_stamp_input(SWEventData *data);
SWSnapshot sw_snapshot(SWWindow *window);
double sw_desktop_top(void);
void sw_frame_invalidate(SWWindow *owner);
void sw_frame_geometry(SWWindow *owner);
BOOL sw_frame_pointer(SWView *view, NSEvent *event);

#endif
