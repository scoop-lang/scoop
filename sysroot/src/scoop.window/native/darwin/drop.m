#import "window.h"
#include <math.h>
#include <string.h>

typedef int32_t (*SWDropCallback)(void *, int32_t, int32_t, int32_t, int32_t,
                                  double, double, int64_t, int32_t, void *);
typedef struct {
  const uint8_t *bytes;
  int64_t length;
} SWDropBytes;

@interface SWDropPayload : NSObject
@property(nonatomic, strong) NSArray<NSData *> *items;
@end
@implementation SWDropPayload
@end

@interface SWDropTarget : NSObject
@property(nonatomic, weak) SWView *view;
@property(nonatomic) SWDropCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) BOOL enabled, closed, inside, inCallback, receiving;
@property(nonatomic) uint64_t revision;
@property(nonatomic) NSInteger sequence;
@property(nonatomic) int32_t choice;
- (int32_t)query:(id<NSDraggingInfo>)info;
- (void)finish;
@end

static int32_t effects(NSDragOperation value) {
  return ((value & NSDragOperationCopy) ? 1 : 0) |
         ((value & NSDragOperationMove) ? 2 : 0) |
         ((value & NSDragOperationLink) ? 4 : 0);
}

static NSDragOperation operation(int32_t choice) {
  switch (choice & 255) {
  case 1:
    return NSDragOperationCopy;
  case 2:
    return NSDragOperationMove;
  case 4:
    return NSDragOperationLink;
  default:
    return NSDragOperationNone;
  }
}

static int32_t formats(NSPasteboard *board) {
  NSArray<NSPasteboardItem *> *items = board.pasteboardItems;
  int32_t result = 0;
  if (items.count == 1) {
    NSArray *types = items.firstObject.types;
    if ([types containsObject:NSPasteboardTypeString])
      result |= 1;
    if ([types containsObject:NSPasteboardTypeURL])
      result |= 2;
  }
  if (items.count) {
    BOOL files = YES;
    for (NSPasteboardItem *item in items)
      files &= [item.types containsObject:NSPasteboardTypeFileURL];
    if (files)
      result |= 4;
  }
  return result;
}

static BOOL valid(int32_t choice, int32_t available, int32_t allowed) {
  int32_t format = choice / 256, effect = choice & 255;
  return format >= 1 && format <= 3 && (available & (1 << (format - 1))) &&
         (effect == 1 || effect == 2 || effect == 4) && (allowed & effect);
}

static SWDropPayload *read_payload(NSPasteboard *board, int32_t format) {
  NSMutableArray<NSData *> *values = [NSMutableArray new];
  if (format == 1 || format == 2) {
    if (board.pasteboardItems.count != 1)
      return nil;
    NSString *value = [board stringForType:format == 1 ? NSPasteboardTypeString
                                                       : NSPasteboardTypeURL];
    if (!value)
      return nil;
    if (format == 2 && ![NSURL URLWithString:value].scheme.length)
      return nil;
    NSData *bytes = [value dataUsingEncoding:NSUTF8StringEncoding];
    if (!bytes)
      return nil;
    [values addObject:bytes];
  } else if (format == 3) {
    NSArray<NSURL *> *urls =
        [board readObjectsForClasses:@[ NSURL.class ]
                             options:@{
                               NSPasteboardURLReadingFileURLsOnlyKey : @YES
                             }];
    if (!urls.count || urls.count != board.pasteboardItems.count)
      return nil;
    for (NSURL *url in urls) {
      const char *path = url.fileSystemRepresentation;
      if (!url.isFileURL || !path || !path[0])
        return nil;
      [values addObject:[NSData dataWithBytes:path length:strlen(path)]];
    }
  } else {
    return nil;
  }
  SWDropPayload *result = [SWDropPayload new];
  result.items = values;
  return result;
}

@implementation SWDropTarget
- (BOOL)available {
  SWView *view = self.view;
  return !self.closed && self.enabled && self.callback && view &&
         view.dropTarget == self && view.window.visible &&
         !view.window.ignoresMouseEvents && !view.owner.inputBlocked &&
         !view.owner.destroying;
}
- (int32_t)call:(id<NSDraggingInfo>)info
          phase:(int32_t)phase
         choice:(int32_t)choice
        payload:(SWDropPayload *)payload {
  if (!self.callback || self.closed || self.inCallback)
    return 0;
  NSPoint point = [self.view convertPoint:info.draggingLocation fromView:nil];
  if (!isfinite(point.x) || !isfinite(point.y))
    return 0;
  self.inCallback = YES;
  @try {
    return self.callback(
        self.context, phase, formats(info.draggingPasteboard),
        effects(info.draggingSourceOperationMask), info.draggingSource != nil,
        point.x, point.y,
        (int64_t)sw_input_modifiers(self.view, NSEvent.modifierFlags), choice,
        (__bridge void *)payload);
  } @finally {
    self.inCallback = NO;
  }
}
- (int32_t)query:(id<NSDraggingInfo>)info {
  if (![self available] || self.inCallback || !self.inside ||
      self.sequence != info.draggingSequenceNumber)
    return 0;
  uint64_t revision = self.revision;
  NSPasteboard *board = info.draggingPasteboard;
  NSInteger change = board.changeCount;
  int32_t choice = [self call:info phase:0 choice:0 payload:nil];
  return [self available] && self.inside &&
                 self.sequence == info.draggingSequenceNumber &&
                 revision == self.revision && board.changeCount == change &&
                 valid(choice, formats(board),
                       effects(info.draggingSourceOperationMask))
             ? choice
             : 0;
}
- (void)finish {
  BOOL notify = self.inside;
  self.inside = NO;
  self.choice = 0;
  if (notify && self.callback && !self.closed && !self.inCallback) {
    self.inCallback = YES;
    @try {
      self.callback(self.context, 2, 0, 0, 0, 0, 0, 0, 0, NULL);
    } @finally {
      self.inCallback = NO;
    }
  }
}
@end

@implementation SWView (DropDestination)
- (NSDragOperation)draggingEntered:(id<NSDraggingInfo>)info {
  @try {
    SWDropTarget *target = self.dropTarget;
    if (target.inCallback || target.receiving)
      return NSDragOperationNone;
    if (![target available]) {
      [target finish];
      return NSDragOperationNone;
    }
    if (target.inside && target.sequence != info.draggingSequenceNumber)
      [target finish];
    target.inside = YES;
    target.sequence = info.draggingSequenceNumber;
    target.choice = [target query:info];
    return operation(target.choice);
  } @catch (NSException *exception) {
    [self.dropTarget finish];
    return NSDragOperationNone;
  }
}
- (NSDragOperation)draggingUpdated:(id<NSDraggingInfo>)info {
  return [self draggingEntered:info];
}
- (BOOL)wantsPeriodicDraggingUpdates {
  return NO;
}
- (BOOL)prepareForDragOperation:(id<NSDraggingInfo>)info {
  @try {
    SWDropTarget *target = self.dropTarget;
    if (target.inCallback || target.receiving)
      return NO;
    if (!target.inside || target.sequence != info.draggingSequenceNumber)
      return NO;
    int32_t previous = target.choice;
    target.choice = [target query:info];
    if (previous && previous == target.choice)
      return YES;
    target.choice = 0;
    return NO;
  } @catch (NSException *exception) {
    [self.dropTarget finish];
    return NO;
  }
}
- (BOOL)performDragOperation:(id<NSDraggingInfo>)info {
  SWDropTarget *target = self.dropTarget;
  if (target.inCallback || target.receiving)
    return NO;
  target.receiving = YES;
  @try {
    int32_t choice = target.choice;
    if (!target.inside || target.sequence != info.draggingSequenceNumber ||
        !choice || [target query:info] != choice || target.choice != choice) {
      [target finish];
      return NO;
    }
    uint64_t revision = target.revision;
    NSPasteboard *board = info.draggingPasteboard;
    NSInteger change = board.changeCount;
    SWDropPayload *payload = read_payload(board, choice / 256);
    if (!payload || ![target available] || target.revision != revision ||
        target.choice != choice || board.changeCount != change) {
      [target finish];
      return NO;
    }
    // Consuming the proposal prevents a nested perform from inserting twice.
    target.choice = 0;
    BOOL accepted = [target call:info phase:1 choice:choice
                         payload:payload] == 1;
    [target finish];
    return accepted;
  } @catch (NSException *exception) {
    [target finish];
    return NO;
  } @finally {
    target.receiving = NO;
  }
}
- (void)draggingExited:(id<NSDraggingInfo>)info {
  SWDropTarget *target = self.dropTarget;
  if (target.inCallback || target.receiving)
    return;
  if (!info || target.sequence == info.draggingSequenceNumber)
    [target finish];
}
- (void)draggingEnded:(id<NSDraggingInfo>)info {
  [self draggingExited:info];
}
- (void)concludeDragOperation:(id<NSDraggingInfo>)info {
  [self draggingExited:info];
}
@end

SWResult scoop_window_drop_create(void *pointer, SWDropCallback callback,
                                  void *context) {
  @try {
    SWWindow *owner = (__bridge SWWindow *)pointer;
    if (!owner.view || owner.destroying)
      return (SWResult){NULL, -4};
    if (owner.view.dropTarget)
      return (SWResult){NULL, -5};
    SWDropTarget *target = [SWDropTarget new];
    target.view = owner.view;
    target.callback = callback;
    target.context = context;
    target.enabled = YES;
    [owner.view registerForDraggedTypes:@[
      NSPasteboardTypeString, NSPasteboardTypeURL, NSPasteboardTypeFileURL
    ]];
    owner.view.dropTarget = target;
    return (SWResult){(__bridge_retained void *)target, 0};
  } @catch (NSException *exception) {
    return (SWResult){NULL, 3};
  }
}

void scoop_window_drop_update(void *pointer, int32_t enabled) {
  SWDropTarget *target = (__bridge SWDropTarget *)pointer;
  target.enabled = enabled != 0;
  target.revision += 1;
  target.choice = 0;
}
void scoop_window_drop_leave(void *pointer) {
  [(__bridge SWDropTarget *)pointer finish];
}
void scoop_window_drop_close(void *pointer) {
  SWDropTarget *target = (__bridge_transfer SWDropTarget *)pointer;
  target.closed = YES;
  target.callback = NULL;
  target.context = NULL;
  target.inside = NO;
  target.choice = 0;
  if (target.view.dropTarget == target) {
    [target.view unregisterDraggedTypes];
    target.view.dropTarget = nil;
  }
}
int64_t scoop_window_drop_item_count(void *pointer) {
  return (int64_t)((__bridge SWDropPayload *)pointer).items.count;
}
SWDropBytes scoop_window_drop_text(void *pointer, int64_t index) {
  SWDropPayload *payload = (__bridge SWDropPayload *)pointer;
  if (index < 0 || (uint64_t)index >= payload.items.count)
    return (SWDropBytes){NULL, -1};
  NSData *item = payload.items[(NSUInteger)index];
  if (item.length > INT64_MAX)
    return (SWDropBytes){NULL, -1};
  return (SWDropBytes){item.bytes, (int64_t)item.length};
}
SWDropBytes scoop_window_drop_path(void *pointer, int64_t index) {
  return scoop_window_drop_text(pointer, index);
}
