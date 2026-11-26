#import "text.h"
#import <QuartzCore/CAMetalLayer.h>
#include <time.h>

@implementation SWIView {
  NSMapTable<UITouch *, NSNumber *> *_contacts;
  int64_t _nextContact;
}
- (instancetype)initWithFrame:(CGRect)frame {
  self = [super initWithFrame:frame];
  if (self) {
    _contacts = [NSMapTable
        mapTableWithKeyOptions:NSPointerFunctionsStrongMemory |
                               NSPointerFunctionsObjectPointerPersonality
                  valueOptions:NSPointerFunctionsStrongMemory];
  }
  return self;
}
+ (Class)layerClass {
  return CAMetalLayer.class;
}
- (void)layoutSubviews {
  [super layoutSubviews];
  [self.owner geometryChanged];
  [self.editor updateKeyboardGeometry];
}
- (void)safeAreaInsetsDidChange {
  [super safeAreaInsetsDidChange];
  [self.owner geometryChanged];
}
- (void)enqueueContact:(SWContactData)data {
  struct timespec now;
  if (clock_gettime(CLOCK_MONOTONIC, &now))
    [NSException raise:NSInternalInconsistencyException
                format:@"Cannot read the input clock"];
  SWIEvent *event = [SWIEvent new];
  event.header = (SWEventHeader){.window = self.owner.identity,
                                 .kind = SWContact,
                                 .time_seconds = now.tv_sec,
                                 .time_nanoseconds = now.tv_nsec};
  event.contact = data;
  [self.owner.application enqueue:event];
}
- (void)deliverTouches:(NSSet<UITouch *> *)touches
                 event:(UIEvent *)event
                 phase:(SWContactPhase)phase {
  for (UITouch *touch in touches) {
    if (touch.type != UITouchTypeDirect && touch.type != UITouchTypePencil)
      continue;
    NSNumber *identity = [_contacts objectForKey:touch];
    if (phase == SWContactBegan) {
      if (identity)
        continue;
      if (_nextContact == INT64_MAX)
        [NSException raise:NSInternalInconsistencyException
                    format:@"Contact identity exhausted"];
      identity = @(++_nextContact);
      [_contacts setObject:identity forKey:touch];
    } else if (!identity)
      continue;
    CGPoint point = [touch locationInView:self];
    CGFloat scale = self.traitCollection.displayScale;
    BOOL pressure = touch.maximumPossibleForce > 0 &&
                    (touch.type == UITouchTypePencil ||
                     self.traitCollection.forceTouchCapability ==
                         UIForceTouchCapabilityAvailable);
    [self
        enqueueContact:(SWContactData){
                           .id = identity.longLongValue,
                           .kind = touch.type == UITouchTypePencil
                                       ? SWContactPen
                                       : SWContactTouch,
                           .phase = phase,
                           .cancel_reason = SWPointerCaptureLost,
                           .modifiers = event.modifierFlags,
                           .x = point.x,
                           .y = point.y,
                           .physical_x = point.x * scale,
                           .physical_y = point.y * scale,
                           .scale = scale,
                           .has_pressure = pressure,
                           .pressure =
                               pressure
                                   ? fmax(0,
                                          fmin(1,
                                               touch.force /
                                                   touch.maximumPossibleForce))
                                   : 0}];
    if (phase == SWContactEnded || phase == SWContactCancelled)
      [_contacts removeObjectForKey:touch];
  }
}
- (void)touchesBegan:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  [self deliverTouches:touches event:event phase:SWContactBegan];
}
- (void)touchesMoved:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  [self deliverTouches:touches event:event phase:SWContactMoved];
}
- (void)touchesEnded:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  [self deliverTouches:touches event:event phase:SWContactEnded];
}
- (void)touchesCancelled:(NSSet<UITouch *> *)touches
               withEvent:(UIEvent *)event {
  [self deliverTouches:touches event:event phase:SWContactCancelled];
}
- (void)cancelContacts:(SWPointerCancelReason)reason {
  for (NSNumber *identity in _contacts.objectEnumerator)
    [self enqueueContact:(SWContactData){.id = identity.longLongValue,
                                         .phase = SWContactCancelled,
                                         .cancel_reason = reason}];
  [_contacts removeAllObjects];
}
@end

SWContactData scoop_window_event_contact(void *pointer) {
  return ((__bridge SWIEvent *)pointer).contact;
}
