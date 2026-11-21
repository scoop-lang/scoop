#import "drag.h"

typedef int32_t (*SWDragCallback)(void *, int32_t, int32_t);

@interface SWDragSource : NSObject <NSDraggingSource>
@property(nonatomic, strong) SWWindow *owner;
@property(nonatomic, strong) NSEvent *event;
@property(nonatomic, strong) NSEvent *press;
@property(nonatomic, strong) NSMutableArray<NSDraggingItem *> *items;
@property(nonatomic, strong) NSDraggingSession *session;
@property(nonatomic, strong) NSImage *preview;
@property(nonatomic) NSPoint hotSpot;
// The OS may retain this source after the managed window and callback retire.
@property(nonatomic, strong) SWDragSource *activeOwner;
@property(nonatomic) SWDragCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) int32_t effects, initialEffects, format;
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
  if (!(self.initialEffects & effect))
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
  self.press = nil;
  self.items = nil;
  self.preview = nil;
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
  if (!event || !delivery.dragPress ||
      delivery.header.window != owner.identity ||
      event.window != owner.window ||
      (event.type != NSEventTypeLeftMouseDown &&
       event.type != NSEventTypeLeftMouseDragged))
    return (SWResult){NULL, -4};
  if (owner.application.dragSource)
    return (SWResult){NULL, -5};
  SWDragSource *source = [SWDragSource new];
  source.owner = owner;
  if (!available(source))
    return (SWResult){NULL, -4};
  source.event = event;
  source.press = delivery.dragPress;
  source.effects = effects;
  source.initialEffects = effects;
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

int64_t scoop_window_drag_preview(void *pointer, int32_t width, int32_t height,
                                  const uint8_t *pixels, double logical_width,
                                  double logical_height, double hot_x,
                                  double hot_y) {
  SWDragSource *source = (__bridge SWDragSource *)pointer;
  @autoreleasepool {
    NSBitmapImageRep *bitmap = sw_rgba_bitmap(
        width, height, pixels, NSMakeSize(logical_width, logical_height));
    if (!bitmap)
      return -7;
    source.preview = [[NSImage alloc] initWithSize:bitmap.size];
    [source.preview addRepresentation:bitmap];
    source.hotSpot = NSMakePoint(hot_x, hot_y);
    return 0;
  }
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
  if (source.owner.view.leftPress == source.press)
    [source.owner.view.pressedButtons removeAllObjects];
  return 0;
}

int64_t scoop_window_drag_effects(void *pointer, int32_t effects) {
  SWDragSource *source = (__bridge SWDragSource *)pointer;
  if (source.closed || (!source.ready && !source.active))
    return -4;
  if (effects < 0 || (effects & ~source.initialEffects))
    return -8;
  source.effects = effects;
  return 0;
}

void sw_drag_item_images(NSArray<NSDraggingItem *> *items, NSImage *preview,
                         NSPoint hotSpot, NSPoint point) {
  if (preview) {
    NSSize size = preview.size;
    NSRect frame = NSMakeRect(point.x - hotSpot.x, point.y - hotSpot.y,
                              size.width, size.height);
    [items.firstObject setDraggingFrame:frame contents:preview];
    // The preview represents the whole selection; keep every pasteboard writer.
    if (items.count > 1) {
      NSImage *clear = [[NSImage alloc] initWithSize:NSMakeSize(1, 1)];
      for (NSUInteger i = 1; i < items.count; ++i)
        [items[i] setDraggingFrame:NSMakeRect(point.x, point.y, 1, 1)
                          contents:clear];
    }
  } else {
    NSImage *image = [NSImage imageNamed:NSImageNameMultipleDocuments];
    NSUInteger index = 0;
    for (NSDraggingItem *item in items) {
      double offset = 4.0 * (double)index++;
      [item setDraggingFrame:NSMakeRect(point.x - 16.0 + offset,
                                        point.y - 16.0 + offset, 32.0, 32.0)
                    contents:image];
    }
  }
}

int64_t scoop_window_drag_start(void *pointer) {
  SWDragSource *source = (__bridge SWDragSource *)pointer;
  if (source.closed || !source.ready || source.active || !available(source))
    return -4;
  SWView *view = source.owner.view;
  if (view.leftPress != source.press) {
    [source releaseData];
    if (source.callback)
      source.callback(source.context, 1, 0);
    return 0;
  }
  view.leftPress = nil;
  NSPoint point = [view convertPoint:source.event.locationInWindow
                            fromView:nil];
  sw_drag_item_images(source.items, source.preview, source.hotSpot, point);
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
