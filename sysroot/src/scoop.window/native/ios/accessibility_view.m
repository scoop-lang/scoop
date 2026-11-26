#import "accessibility.h"
#import "window.h"

@class SWIAXElement;
@interface SWIAXProjection : NSObject
@property(nonatomic, weak) SWIScene *scene;
@property(nonatomic) void *tree;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWIAXElement *> *elements;
@end
@implementation SWIAXProjection
@end

@interface SWIAXElement : UIAccessibilityElement
@property(nonatomic, weak) SWIAXProjection *projection;
@property(nonatomic) SWAccessibilityData node;
@end

@implementation SWIAXElement
- (BOOL)accessibilityActivate {
  int actions = self.node.actions;
  int action;
  if (actions & 1)
    action = 0;
  else if (actions & 2)
    action = 1;
  else if (actions & 32768)
    action = self.node.expanded == 1 ? 18 : 17;
  else if (actions & 16384)
    action = 15;
  else if (actions & 8)
    action = 3;
  else
    return NO;
  return swi_ax_perform(self.projection.tree, self.node.number, action);
}
- (void)accessibilityIncrement {
  swi_ax_perform(self.projection.tree, self.node.number, 13);
}
- (void)accessibilityDecrement {
  swi_ax_perform(self.projection.tree, self.node.number, 14);
}
- (BOOL)accessibilityPerformEscape {
  return swi_ax_perform(self.projection.tree, self.node.number, 19);
}
- (BOOL)accessibilityScroll:(UIAccessibilityScrollDirection)direction {
  int axis, forward;
  switch (direction) {
  case UIAccessibilityScrollDirectionUp:
    axis = 2;
    forward = 0;
    break;
  case UIAccessibilityScrollDirectionDown:
    axis = 2;
    forward = 1;
    break;
  case UIAccessibilityScrollDirectionLeft:
    axis = 1;
    forward = 0;
    break;
  case UIAccessibilityScrollDirectionRight:
    axis = 1;
    forward = 1;
    break;
  case UIAccessibilityScrollDirectionNext:
    axis = -1;
    forward = 1;
    break;
  case UIAccessibilityScrollDirectionPrevious:
    axis = -1;
    forward = 0;
    break;
  default:
    return NO;
  }
  if (!swi_ax_scroll(self.projection.tree, self.node.number, axis, forward))
    return NO;
  UIAccessibilityPostNotification(UIAccessibilityPageScrolledNotification, nil);
  return YES;
}
- (CGPoint)accessibilityActivationPoint {
  SWIView *view = self.projection.scene.view;
  if (self.node.activation && view.window.windowScene) {
    CGPoint point = CGPointMake(self.node.activation_x, self.node.activation_y);
    return [view convertPoint:point
            toCoordinateSpace:view.window.windowScene.screen.coordinateSpace];
  }
  return [super accessibilityActivationPoint];
}
@end

static NSString *string(SWAccessibilityBytes field) {
  return field.bytes ? [[NSString alloc] initWithBytes:field.bytes
                                                length:(NSUInteger)field.length
                                              encoding:NSUTF8StringEncoding]
                     : nil;
}

static UIAccessibilityTraits traits(SWAccessibilityData d) {
  UIAccessibilityTraits value = UIAccessibilityTraitNone;
  switch (d.role) {
  case 1:
    value |= UIAccessibilityTraitStaticText;
    break;
  case 2:
  case 9:
  case 18:
  case 20:
  case 26:
    value |= UIAccessibilityTraitButton;
    break;
  case 3:
  case 7:
  case 8:
    value |= UIAccessibilityTraitButton | UIAccessibilityTraitToggleButton;
    break;
  case 11:
    value |= UIAccessibilityTraitImage;
    break;
  case 13:
    value |= UIAccessibilityTraitLink;
    break;
  case 14:
  case 24:
  case 25:
    value |= UIAccessibilityTraitHeader;
    break;
  case 19:
    value |= UIAccessibilityTraitTabBar;
    break;
  default:
    break;
  }
  if (d.actions & (4096 | 8192))
    value |= UIAccessibilityTraitAdjustable;
  if (d.selected == 1 || d.checked == 1)
    value |= UIAccessibilityTraitSelected;
  if (!(d.flags & SWAXEnabled))
    value |= UIAccessibilityTraitNotEnabled;
  return value;
}

void *swi_ax_attach(void *scene, void *tree) {
  SWIAXProjection *p = [SWIAXProjection new];
  p.scene = (__bridge SWIScene *)scene;
  p.tree = tree;
  p.elements = [NSMutableDictionary dictionary];
  p.scene.view.isAccessibilityElement = NO;
  p.scene.view.accessibilityElements = @[];
  return (__bridge_retained void *)p;
}
void swi_ax_detach(void *handle) {
  SWIAXProjection *p = CFBridgingRelease(handle);
  p.tree = NULL;
  p.scene.view.accessibilityElements = nil;
  [p.elements removeAllObjects];
}
int swi_ax_on_main(void) { return NSThread.isMainThread; }
int swi_ax_query(void *handle, enum SWIAccessibilityQuery query) {
  SWIScene *scene = ((__bridge SWIAXProjection *)handle).scene;
  if (!scene)
    return 0;
  switch (query) {
  case SWIVisible:
    return scene.foreground && !scene.window.hidden;
  case SWIFocused:
    return scene.active && scene.window.isKeyWindow;
  case SWIEnabled:
    return scene.active && scene.view.userInteractionEnabled;
  }
}
int swi_ax_geometry(void *handle, enum SWIAccessibilitySpace space,
                    SWIAccessibilityRect *rect) {
  UIView *view = ((__bridge SWIAXProjection *)handle).scene.view;
  id<UICoordinateSpace> screen = view.window.windowScene.screen.coordinateSpace;
  if (!screen)
    return 0;
  CGRect frame = CGRectMake(rect->x, rect->y, rect->width, rect->height);
  switch (space) {
  case SWIViewBounds:
    frame = view.bounds;
    break;
  case SWIToScreen:
    frame = [view convertRect:frame toCoordinateSpace:screen];
    break;
  case SWIFromScreen:
    frame = [view convertRect:frame fromCoordinateSpace:screen];
    break;
  }
  *rect = (SWIAccessibilityRect){frame.origin.x, frame.origin.y,
                                 frame.size.width, frame.size.height};
  return 1;
}
void swi_ax_update(void *handle, const SWAccessibilityData *data,
                   const SWAccessibilityBytes fields[6]) {
  SWIAXProjection *p = (__bridge SWIAXProjection *)handle;
  SWIAXElement *element = p.elements[@(data->number)];
  if (!element) {
    element =
        [[SWIAXElement alloc] initWithAccessibilityContainer:p.scene.view];
    element.projection = p;
    p.elements[@(data->number)] = element;
  }
  element.node = *data;
  element.accessibilityLabel = string(fields[0]);
  element.accessibilityHint = string(fields[4]) ?: string(fields[1]);
  element.accessibilityIdentifier = string(fields[2]);
  element.accessibilityValue =
      (data->flags & SWAXProtected) ? nil : string(fields[3]);
  if (data->has_range)
    element.accessibilityValue = [NSNumberFormatter
        localizedStringFromNumber:@(data->number_value)
                      numberStyle:NSNumberFormatterDecimalStyle];
  element.accessibilityTraits = traits(*data);
  element.accessibilityFrameInContainerSpace =
      CGRectMake(data->visible_x, data->visible_y, data->visible_width,
                 data->visible_height);
}
void swi_ax_remove(void *handle, int64_t number) {
  SWIAXProjection *p = (__bridge SWIAXProjection *)handle;
  p.elements[@(number)].projection = nil;
  [p.elements removeObjectForKey:@(number)];
}
void swi_ax_order(void *handle, const int64_t *numbers, size_t count) {
  SWIAXProjection *p = (__bridge SWIAXProjection *)handle;
  NSMutableArray *order = [NSMutableArray arrayWithCapacity:count];
  for (size_t i = 0; i < count; ++i)
    [order addObject:p.elements[@(numbers[i])]];
  if (![p.scene.view.accessibilityElements isEqualToArray:order]) {
    p.scene.view.accessibilityElements = order;
    UIAccessibilityPostNotification(UIAccessibilityLayoutChangedNotification,
                                    nil);
  }
}
void swi_ax_announce(void *handle, SWAccessibilityBytes text, int priority) {
  if (!swi_ax_query(handle, SWIVisible))
    return;
  NSString *message = string(text);
  if (!message.length)
    return;
  NSAttributedString *announcement = [[NSAttributedString alloc]
      initWithString:message
          attributes:@{
            UIAccessibilitySpeechAttributeQueueAnnouncement : @(priority == 1)
          }];
  UIAccessibilityPostNotification(UIAccessibilityAnnouncementNotification,
                                  announcement);
}
