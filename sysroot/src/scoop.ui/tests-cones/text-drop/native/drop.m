#import "../../../../scoop.window/native/darwin/window.h"
#import <AppKit/AppKit.h>
#include <stdint.h>

@interface SUTextDrop : NSObject <NSDraggingInfo>
@property(nonatomic, strong) NSView *view;
@property(nonatomic) BOOL entered;
@property(nonatomic) BOOL closed;
@property(nonatomic, strong) NSWindow *draggingDestinationWindow;
@property(nonatomic) NSDragOperation draggingSourceOperationMask;
@property(nonatomic) NSPoint draggingLocation, draggedImageLocation;
@property(nonatomic, strong) NSImage *draggedImage;
@property(nonatomic, strong) NSPasteboard *draggingPasteboard;
@property(nonatomic, weak) id draggingSource;
@property(nonatomic) NSInteger draggingSequenceNumber;
@property NSInteger numberOfValidItemsForDrop;
@property NSDraggingFormation draggingFormation;
@property BOOL animatesToDestination;
@property(nonatomic) NSSpringLoadingHighlight springLoadingHighlight;
@end

// Test-only declarations for the library's prepared source; no OS session is
// started.
@interface SWDragSource : NSObject <NSDraggingSource>
@property(nonatomic, readonly) NSArray<NSDraggingItem *> *items;
@property(nonatomic, readonly) BOOL active;
- (NSDragOperation)draggingSession:(NSDraggingSession *_Nullable)session
    sourceOperationMaskForDraggingContext:(NSDraggingContext)context;
@end

@implementation SUTextDrop
- (void)slideDraggedImageTo:(NSPoint)point {
  self.draggedImageLocation = point;
}
- (NSArray<NSString *> *)namesOfPromisedFilesDroppedAtDestination:(NSURL *)url {
  return nil;
}
- (void)enumerateDraggingItemsWithOptions:
            (NSDraggingItemEnumerationOptions)options
                                  forView:(NSView *)view
                                  classes:(NSArray<Class> *)classes
                            searchOptions:(NSDictionary *)searchOptions
                               usingBlock:(void (^)(NSDraggingItem *, NSInteger,
                                                    BOOL *))block {
  NSArray *items =
      [self.draggingPasteboard readObjectsForClasses:classes
                                             options:searchOptions];
  BOOL stop = NO;
  NSInteger index = 0;
  for (id item in items) {
    block([[NSDraggingItem alloc] initWithPasteboardWriter:item], index++,
          &stop);
    if (stop)
      break;
  }
}
- (void)resetSpringLoading {
  self.springLoadingHighlight = NSSpringLoadingHighlightNone;
}
@end

void *scoop_ui_text_drop_open(void *view) {
  SUTextDrop *probe = [SUTextDrop new];
  probe.view = (__bridge NSView *)view;
  probe.draggingDestinationWindow = probe.view.window;
  return (__bridge_retained void *)probe;
}

void scoop_ui_text_drop_post(void *pointer, int32_t (*callback)(void *),
                             void *context) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!probe.closed)
      callback(context);
  });
}

void scoop_ui_text_drop_leave(void *pointer) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  [probe.view draggingExited:probe];
  probe.entered = NO;
}

bool scoop_ui_text_drop_content(void *pointer, const uint8_t *bytes,
                                int64_t length, int32_t format) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  scoop_ui_text_drop_leave(pointer);
  [probe.draggingPasteboard releaseGlobally];
  probe.draggingPasteboard = [NSPasteboard pasteboardWithUniqueName];
  probe.draggingSequenceNumber += 1;
  NSString *text = [[NSString alloc] initWithBytes:bytes
                                            length:(NSUInteger)length
                                          encoding:NSUTF8StringEncoding];
  if (!text)
    return false;
  NSPasteboardType type =
      format == 1 ? NSPasteboardTypeURL : NSPasteboardTypeString;
  return [probe.draggingPasteboard setString:text forType:type];
}

int32_t scoop_ui_text_drop_offer(void *pointer, double x, double y,
                                 int32_t effects) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  probe.draggingLocation = [probe.view convertPoint:NSMakePoint(x, y)
                                             toView:nil];
  probe.draggingSourceOperationMask =
      ((effects & 1) ? NSDragOperationCopy : 0) |
      ((effects & 2) ? NSDragOperationMove : 0) |
      ((effects & 4) ? NSDragOperationLink : 0);
  NSDragOperation result = probe.entered ? [probe.view draggingUpdated:probe]
                                         : [probe.view draggingEntered:probe];
  probe.entered = YES;
  return (int32_t)result;
}

int32_t scoop_ui_text_drop_perform(void *pointer) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  int32_t result = [probe.view prepareForDragOperation:probe] ? 1 : 0;
  if ([probe.view performDragOperation:probe])
    result |= 2;
  if ([probe.view performDragOperation:probe])
    result |= 4;
  [probe.view concludeDragOperation:probe];
  [probe.view draggingEnded:probe];
  scoop_ui_text_drop_leave(pointer);
  return result;
}

bool scoop_ui_text_drop_mark(void *pointer) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  id<NSTextInputClient> client = (id<NSTextInputClient>)probe.view;
  [client setMarkedText:@"候"
          selectedRange:NSMakeRange(1, 0)
       replacementRange:NSMakeRange(NSNotFound, 0)];
  return client.hasMarkedText;
}

bool scoop_ui_text_drop_can_mark(void *pointer) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  id<NSTextInputClient> client = (id<NSTextInputClient>)probe.view;
  return probe.view.window.keyWindow &&
         probe.view.window.firstResponder == probe.view &&
         probe.view.inputContext &&
         NSEqualRanges(client.selectedRange, NSMakeRange(1, 0));
}

void scoop_ui_text_drop_close(void *pointer) {
  SUTextDrop *probe = (__bridge_transfer SUTextDrop *)pointer;
  probe.closed = YES;
  [probe.view draggingExited:probe];
  [probe.draggingPasteboard releaseGlobally];
}

void scoop_ui_text_source_input(void *pointer, int32_t kind, double x, double y,
                                int32_t count, bool shift) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (probe.closed || !probe.view.window)
      return;
    NSEventType type = kind == 0   ? NSEventTypeLeftMouseDown
                       : kind == 1 ? NSEventTypeLeftMouseDragged
                                   : NSEventTypeLeftMouseUp;
    NSEvent *event =
        [NSEvent mouseEventWithType:type
                           location:[probe.view convertPoint:NSMakePoint(x, y)
                                                      toView:nil]
                      modifierFlags:shift ? NSEventModifierFlagShift : 0
                          timestamp:NSProcessInfo.processInfo.systemUptime
                       windowNumber:probe.view.window.windowNumber
                            context:nil
                        eventNumber:200 + kind
                         clickCount:count
                           pressure:kind == 2 ? 0 : 1];
    if (kind == 0)
      [probe.view mouseDown:event];
    else if (kind == 1)
      [probe.view mouseDragged:event];
    else
      [probe.view mouseUp:event];
  });
}

int32_t scoop_ui_text_source_mask(void *pointer) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  SWView *view = (SWView *)probe.view;
  SWDragSource *source = view.owner.application.dragSource;
  return (int32_t)[source draggingSession:nil
      sourceOperationMaskForDraggingContext:NSDraggingContextWithinApplication];
}

bool scoop_ui_text_source_ready(void *pointer, const uint8_t *bytes,
                                int64_t length) {
  SUTextDrop *probe = (__bridge SUTextDrop *)pointer;
  SWView *view = (SWView *)probe.view;
  SWDragSource *source = view.owner.application.dragSource;
  if (!source || source.active || source.items.count != 1)
    return false;
  NSPasteboardItem *item = (id)source.items.firstObject.item;
  NSString *expected = [[NSString alloc] initWithBytes:bytes
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
  return [[item stringForType:NSPasteboardTypeString] isEqualToString:expected];
}
