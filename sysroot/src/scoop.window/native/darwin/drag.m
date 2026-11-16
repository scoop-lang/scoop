#import "drag.h"

typedef int32_t (*SWDragCallback)(void *, int32_t, int32_t);

@interface SWDragSource : NSObject <NSDraggingSource>
@property(nonatomic, strong) SWWindow *owner;
@property(nonatomic, strong) NSEvent *event;
@property(nonatomic, strong) NSMutableArray<NSDraggingItem *> *items;
@property(nonatomic, strong) NSDraggingSession *session;
// The OS may retain this source after the managed window and callback retire.
@property(nonatomic, strong) SWDragSource *activeOwner;
@property(nonatomic) SWDragCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) int32_t effects, format;
@property(nonatomic) BOOL ready, active, started, closed;
- (void)releaseData;
- (void)reportStarted:(NSDraggingSession *)session;
@end

static BOOL available(SWDragSource *source) {
  SWWindow *owner = source.owner;
  return owner && !owner.destroying && owner.application &&
         !owner.inputBlocked && !owner.window.ignoresMouseEvents &&
         owner.view.window == owner.window;
}

@implementation SWDragSource
- (NSDragOperation)draggingSession:(NSDraggingSession *)session
    sourceOperationMaskForDraggingContext:(NSDraggingContext)context {
  if (self.closed || (!self.ready && !self.active))
    return NSDragOperationNone;
  return ((self.effects & 1) ? NSDragOperationCopy : 0) |
         ((self.effects & 2) ? NSDragOperationMove : 0) |
         ((self.effects & 4) ? NSDragOperationLink : 0);
}
- (BOOL)ignoreModifierKeysForDraggingSession:(NSDraggingSession *)session {
  return NO;
}
- (void)reportStarted:(NSDraggingSession *)session {
  if (!self.active || !session || self.started)
    return;
  self.session = session;
  self.started = YES;
  if (self.callback)
    self.callback(self.context, 0, 0);
}
- (void)draggingSession:(NSDraggingSession *)session
       willBeginAtPoint:(NSPoint)point {
  [self reportStarted:session];
}
- (void)draggingSession:(NSDraggingSession *)session
           endedAtPoint:(NSPoint)point
              operation:(NSDragOperation)operation {
  if (!self.active || !session || (self.session && self.session != session))
    return;
  [self reportStarted:session];
  self.active = NO;
  int32_t effect = operation == NSDragOperationCopy   ? 1
                   : operation == NSDragOperationMove ? 2
                   : operation == NSDragOperationLink ? 4
                                                      : 0;
  if (!(self.effects & effect))
    effect = 0;
  [self releaseData];
  if (self.callback)
    self.callback(self.context, 1, effect);
  self.activeOwner = nil;
}
- (void)releaseData {
  if (self.owner.application.dragSource == self)
    self.owner.application.dragSource = nil;
  self.owner = nil;
  self.event = nil;
  self.items = nil;
  self.session = nil;
  self.ready = NO;
}
@end

SWResult scoop_window_drag_create(void *pointer, void *input, int32_t effects,
                                  SWDragCallback callback, void *context) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  SWEvent *delivery = (__bridge SWEvent *)input;
  NSEvent *event = delivery.nativeInput;
  if (!callback || effects <= 0 || effects > 7)
    return (SWResult){NULL, -8};
  if (!event || delivery.data.window != owner.identity ||
      event.window != owner.window ||
      (event.type != NSEventTypeLeftMouseDown &&
       event.type != NSEventTypeLeftMouseDragged) ||
      ![owner.view.pressedButtons containsObject:@0])
    return (SWResult){NULL, -4};
  if (owner.application.dragSource)
    return (SWResult){NULL, -5};
  SWDragSource *source = [SWDragSource new];
  source.owner = owner;
  if (!available(source))
    return (SWResult){NULL, -4};
  source.event = event;
  source.effects = effects;
  source.items = [NSMutableArray new];
  source.callback = callback;
  source.context = context;
  return (SWResult){(__bridge_retained void *)source, 0};
}

int64_t scoop_window_drag_add(void *pointer, int32_t format,
                              const uint8_t *bytes, int64_t length) {
  SWDragSource *source = (__bridge SWDragSource *)pointer;
  if (source.closed || source.ready || source.active)
    return -4;
  if (source.items.count && (format != 3 || source.format != 3))
    return -8;
  NSDraggingItem *item = sw_drag_item(format, bytes, length);
  if (!item)
    return -8;
  source.format = format;
  [source.items addObject:item];
  return 0;
}

int64_t scoop_window_drag_add_path(void *pointer, const uint8_t *bytes,
                                   int64_t length) {
  return scoop_window_drag_add(pointer, 3, bytes, length);
}

int64_t scoop_window_drag_ready(void *pointer) {
  SWDragSource *source = (__bridge SWDragSource *)pointer;
  if (source.closed || source.ready || source.active || !available(source))
    return -4;
  if (source.owner.application.dragSource)
    return -5;
  if (!source.items.count)
    return -8;
  source.ready = YES;
  source.owner.application.dragSource = source;
  [source.owner.view.pressedButtons removeAllObjects];
  return 0;
}

int64_t scoop_window_drag_start(void *pointer) {
  SWDragSource *source = (__bridge SWDragSource *)pointer;
  if (source.closed || !source.ready || source.active || !available(source))
    return -4;
  SWView *view = source.owner.view;
  NSPoint point = [view convertPoint:source.event.locationInWindow
                            fromView:nil];
  NSImage *image = [NSImage imageNamed:NSImageNameMultipleDocuments];
  NSUInteger index = 0;
  for (NSDraggingItem *item in source.items) {
    double offset = 4.0 * (double)index++;
    [item setDraggingFrame:NSMakeRect(point.x - 16.0 + offset,
                                      point.y - 16.0 + offset, 32.0, 32.0)
                  contents:image];
  }
  source.ready = NO;
  source.active = YES;
  source.activeOwner = source;
  @try {
    NSDraggingSession *session =
        [view beginDraggingSessionWithItems:source.items
                                      event:source.event
                                     source:source];
    if (!session) {
      source.active = NO;
      [source releaseData];
      source.activeOwner = nil;
      return -7;
    }
    [source reportStarted:session];
    return 0;
  } @catch (NSException *exception) {
    source.active = NO;
    [source releaseData];
    source.activeOwner = nil;
    return -7;
  }
}

void scoop_window_drag_close(void *pointer) {
  SWDragSource *source = (__bridge_transfer SWDragSource *)pointer;
  source.closed = YES;
  source.callback = NULL;
  source.context = NULL;
  if (!source.active)
    [source releaseData];
}
