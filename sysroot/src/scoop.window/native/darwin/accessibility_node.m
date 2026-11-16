#import "accessibility.h"
#include <math.h>

@implementation SWAccessibilityNode
- (SWAccessibilityState *)state {
  if (![NSThread isMainThread] || self.retired)
    return nil;
  SWAccessibilityTree *tree = self.tree;
  if (!tree || tree.closed || [tree.retiring containsObject:@(self.number)])
    return nil;
  return self.number == 0 ? tree.rootState : tree.published[@(self.number)];
}
- (BOOL)participatesInAccessibility {
  SWAccessibilityState *state = self.state;
  if (!state || !self.tree.view.window.visible ||
      self.tree.view.owner.inputBlocked)
    return NO;
  while (state) {
    if (state.data.flags & SWAXHidden)
      return NO;
    state = state.data.parent ? self.tree.published[@(state.data.parent)] : nil;
  }
  return YES;
}
- (BOOL)isAccessibilityElement {
  return !(self.state.data.flags & SWAXIgnored) &&
         [self participatesInAccessibility];
}
- (NSAccessibilityRole)accessibilityRole {
  SWAccessibilityState *state = self.state;
  if (!state)
    return nil;
  NSAccessibilityRole roles[] = {
      NSAccessibilityGroupRole,     NSAccessibilityStaticTextRole,
      NSAccessibilityButtonRole,    NSAccessibilityCheckBoxRole,
      NSAccessibilityTextFieldRole, NSAccessibilityScrollAreaRole,
      NSAccessibilityScrollBarRole, NSAccessibilityButtonRole,
      NSAccessibilityButtonRole,    NSAccessibilityRadioButtonRole,
      NSAccessibilityRadioGroupRole};
  return roles[state.data.role];
}
- (NSString *)accessibilitySubrole {
  SWAccessibilityState *state = self.state;
  if (state.data.flags & SWAXProtected)
    return NSAccessibilitySecureTextFieldSubrole;
  if (state.data.role == 7)
    return NSAccessibilitySwitchSubrole;
  if (state.data.role == 8)
    return NSAccessibilityToggleSubrole;
  return nil;
}
- (BOOL)usesToggleAction {
  int32_t role = self.state.data.role;
  return role == 3 || role == 7 || role == 8;
}
- (NSString *)accessibilityLabel {
  return self.state.name;
}
- (NSString *)accessibilityTitle {
  return self.state.name;
}
- (NSString *)accessibilityHelp {
  SWAccessibilityState *state = self.state;
  if (!state.help.length)
    return state.error ?: state.help;
  if (!state.error.length)
    return state.help;
  return [NSString stringWithFormat:@"%@\n%@", state.help, state.error];
}
- (id)accessibilityTitleUIElement {
  SWAccessibilityState *state = self.state;
  if (!self.isAccessibilityElement || (state.data.flags & SWAXNamed) ||
      state.labels.count != 1)
    return nil;
  SWAccessibilityNode *label =
      [self.tree element:state.labels[0].longLongValue];
  return label.isAccessibilityElement ? label : nil;
}
- (NSArray *)relatedElements:(NSArray<NSNumber *> *)numbers {
  if (!self.isAccessibilityElement)
    return @[];
  NSMutableArray *result = [NSMutableArray new];
  for (NSNumber *number in numbers) {
    SWAccessibilityNode *node = [self.tree element:number.longLongValue];
    if (node.isAccessibilityElement)
      [result addObject:node];
  }
  return result;
}
- (NSArray *)accessibilityServesAsTitleForUIElements {
  return [[self relatedElements:self.state.titleFor]
      sortedArrayUsingComparator:^NSComparisonResult(
          SWAccessibilityNode *left, SWAccessibilityNode *right) {
        int64_t a = left.state.data.order, b = right.state.data.order;
        if (a != b)
          return a < b ? NSOrderedAscending : NSOrderedDescending;
        return left.number < right.number   ? NSOrderedAscending
               : left.number > right.number ? NSOrderedDescending
                                            : NSOrderedSame;
      }];
}
- (NSArray *)accessibilityLinkedUIElements {
  return [self relatedElements:self.state.controls];
}
- (NSString *)accessibilityIdentifier {
  return self.state.identifier;
}
- (BOOL)isAccessibilityProtectedContent {
  return (self.state.data.flags & SWAXProtected) != 0;
}
- (BOOL)isAccessibilityRequired {
  return (self.state.data.flags & SWAXRequired) != 0;
}
- (id)accessibilityValue {
  SWAccessibilityState *state = self.state;
  if (!state || (state.data.flags & SWAXProtected))
    return nil;
  if (state.data.scroll_orientation) {
    double maximum =
        fmax(state.data.scroll_content - state.data.scroll_viewport, 0);
    return @(maximum > 0 ? state.data.scroll_offset / maximum : 0);
  }
  return state.data.checked >= 0 ? @(state.data.checked) : state.value;
}
- (id)accessibilityMinValue {
  return self.state.data.scroll_orientation ? @0.0 : nil;
}
- (id)accessibilityMaxValue {
  SWAccessibilityData data = self.state.data;
  return data.scroll_orientation
             ? @(data.scroll_content > data.scroll_viewport ? 1.0 : 0.0)
             : nil;
}
- (NSAccessibilityOrientation)accessibilityOrientation {
  switch (self.state.data.scroll_orientation) {
  case 1:
    return NSAccessibilityOrientationHorizontal;
  case 2:
    return NSAccessibilityOrientationVertical;
  default:
    return NSAccessibilityOrientationUnknown;
  }
}
- (id)scrollBarWithOrientation:(int32_t)orientation {
  if (!self.isAccessibilityElement)
    return nil;
  for (NSNumber *number in self.tree.children[@(self.number)]) {
    SWAccessibilityState *state = self.tree.published[number];
    if (state.data.role == 6 && state.data.scroll_orientation == orientation) {
      SWAccessibilityNode *bar = [self.tree element:number.longLongValue];
      if (bar.isAccessibilityElement)
        return bar;
    }
  }
  return nil;
}
- (id)accessibilityHorizontalScrollBar {
  return [self scrollBarWithOrientation:1];
}
- (id)accessibilityVerticalScrollBar {
  return [self scrollBarWithOrientation:2];
}
- (BOOL)isAccessibilityEnabled {
  SWAccessibilityState *state = self.state;
  if (!state || !self.isAccessibilityElement)
    return NO;
  while (state) {
    if (!(state.data.flags & SWAXEnabled))
      return NO;
    state = state.data.parent ? self.tree.published[@(state.data.parent)] : nil;
  }
  return YES;
}
- (id)accessibilityParent {
  SWAccessibilityState *state = self.state;
  if (!state)
    return nil;
  if (self.number == 0)
    return NSAccessibilityUnignoredAncestor(self.tree.view);
  int64_t parent = state.data.parent;
  while (parent) {
    SWAccessibilityState *ancestor = self.tree.published[@(parent)];
    if (!ancestor)
      return nil;
    if (!(ancestor.data.flags & SWAXIgnored))
      break;
    parent = ancestor.data.parent;
  }
  return [self.tree element:parent];
}
- (id)accessibilityWindow {
  return self.state ? self.tree.view.window : nil;
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (void)appendChildrenOf:(int64_t)parent to:(NSMutableArray *)children {
  SWAccessibilityTree *tree = self.tree;
  for (NSNumber *number in tree.children[@(parent)]) {
    SWAccessibilityState *state = tree.published[number];
    if (!state || (state.data.flags & SWAXHidden) ||
        [tree.retiring containsObject:number])
      continue;
    if (state.data.flags & SWAXIgnored)
      [self appendChildrenOf:number.longLongValue to:children];
    else {
      SWAccessibilityNode *child = [tree element:number.longLongValue];
      if (child.isAccessibilityElement)
        [children addObject:child];
    }
  }
}
- (NSArray *)accessibilityChildren {
  if (![self participatesInAccessibility])
    return @[];
  NSMutableArray *children = [NSMutableArray new];
  [self appendChildrenOf:self.number to:children];
  return children;
}
- (NSArray *)accessibilityChildrenInNavigationOrder {
  return self.accessibilityChildren;
}
- (NSArray *)accessibilityVisibleChildren {
  NSMutableArray *visible = [NSMutableArray new];
  for (SWAccessibilityNode *node in self.accessibilityChildren)
    if (node.state.data.geometry && node.state.data.visible)
      [visible addObject:node];
  return visible;
}
- (NSRect)accessibilityFrame {
  SWAccessibilityState *state = self.state;
  SWView *view = self.tree.view;
  if (!state || !view.window)
    return NSZeroRect;
  if (self.number == 0)
    return NSAccessibilityFrameInView(view, view.bounds);
  SWAccessibilityData data = state.data;
  if (!data.geometry)
    return NSZeroRect;
  return NSAccessibilityFrameInView(
      view, NSMakeRect(data.x, data.y, data.width, data.height));
}
- (BOOL)isWithin:(int64_t)ancestor {
  int64_t number = self.number;
  while (number) {
    if (number == ancestor)
      return YES;
    SWAccessibilityState *state = self.tree.published[@(number)];
    if (!state)
      return NO;
    number = state.data.parent;
  }
  return ancestor == 0;
}
- (id)accessibilityHitTest:(NSPoint)screenPoint {
  if (![self participatesInAccessibility] || !isfinite(screenPoint.x) ||
      !isfinite(screenPoint.y))
    return nil;
  SWAccessibilityTree *tree = self.tree;
  SWView *view = tree.view;
  NSPoint windowPoint = [view.window convertPointFromScreen:screenPoint];
  NSPoint point = [view convertPoint:windowPoint fromView:nil];
  if (!NSPointInRect(point, view.bounds))
    return nil;
  for (NSNumber *number in tree.hitOrder.reverseObjectEnumerator) {
    SWAccessibilityNode *node = [tree element:number.longLongValue];
    SWAccessibilityState *state = node.state;
    if (!state || !state.data.geometry || !state.data.visible ||
        ![node participatesInAccessibility] || ![node isWithin:self.number] ||
        ![state.hit containsPoint:point])
      continue;
    while (node.number && (node.state.data.flags & SWAXIgnored))
      node = [tree element:node.state.data.parent];
    if (node.isAccessibilityElement)
      return node;
  }
  return self.number == 0 ? self : nil;
}
- (BOOL)isAccessibilityFocused {
  return self.state && self.number != 0 && self.tree.focused == self.number &&
         self.isAccessibilityElement && self.tree.view.window.keyWindow;
}
- (id)accessibilityFocusedUIElement {
  return self.state ? sw_accessibility_focused(self.tree.view) : nil;
}
- (int32_t)perform:(int32_t)action text:(NSString *)text {
  return [self perform:action text:text range:NSMakeRange(NSNotFound, 0)];
}
- (int32_t)perform:(int32_t)action text:(NSString *)text range:(NSRange)range {
  return [self perform:action text:text range:range position:0];
}
- (int32_t)perform:(int32_t)action
              text:(NSString *)text
             range:(NSRange)range
          position:(double)position {
  SWAccessibilityTree *tree = self.tree;
  SWAccessibilityState *state = self.state;
  if (!state || tree.closed || !tree.callback)
    return 4;
  if (!self.isAccessibilityEnabled)
    return 5;
  NSData *bytes = text ? [text dataUsingEncoding:NSUTF8StringEncoding] : nil;
  if (text && !bytes)
    return 2;
  int64_t anchor = -1, cursor = -1;
  if (action == 5 || action == 6) {
    NSString *body = self.accessibleText;
    if (!body || !sw_text_range(body, range))
      return 2;
    anchor = (int64_t)[[body substringToIndex:range.location]
        lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
    cursor = (int64_t)[[body substringToIndex:NSMaxRange(range)]
        lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
  }
  SWAccessibilityCallback callback = tree.callback;
  void *context = tree.context;
  return callback(context, self.number, action, bytes.bytes,
                  text ? (int64_t)bytes.length : -1, anchor, cursor,
                  state.data.content_revision, position);
}
- (BOOL)accessibilityPerformPress {
  SWAccessibilityState *state = self.state;
  if (!state)
    return NO;
  int32_t action = self.usesToggleAction && (state.data.actions & 2) ? 1 : 0;
  if (!(state.data.actions & (1 << action)))
    return NO;
  return [self perform:action text:nil] == 1;
}
- (void)setAccessibilityValue:(id)value {
  SWAccessibilityState *state = self.state;
  if (state.data.scroll_orientation) {
    if ((state.data.actions & 128) && !(state.data.flags & SWAXReadOnly) &&
        [value isKindOfClass:NSNumber.class])
      [self perform:8
               text:nil
              range:NSMakeRange(NSNotFound, 0)
           position:[value doubleValue]];
    return;
  }
  if (!state || !(state.data.actions & 4) ||
      (state.data.flags & SWAXReadOnly) ||
      ![value isKindOfClass:NSString.class])
    return;
  [self perform:2 text:value];
}
- (BOOL)accessibilityPerformDecrement {
  return self.state.data.scroll_orientation &&
         (self.state.data.actions & 256) && [self perform:9 text:nil] == 1;
}
- (BOOL)accessibilityPerformIncrement {
  return self.state.data.scroll_orientation &&
         (self.state.data.actions & 512) && [self perform:10 text:nil] == 1;
}
- (BOOL)accessibilityPerformShowMenu {
  if (!(self.state.data.actions & 64))
    return NO;
  return [self perform:7 text:nil] == 1;
}
- (void)setAccessibilityFocused:(BOOL)focused {
  if (self.state.data.actions & 8)
    [self perform:focused ? 3 : 4 text:nil];
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  SWAccessibilityState *state = self.state;
  if (!state)
    return NO;
  if (selector == @selector(isAccessibilityRequired))
    return self.isAccessibilityElement;
  if (selector == @selector(accessibilityPerformPress))
    return self.isAccessibilityEnabled &&
           (state.data.actions & (self.usesToggleAction ? 3 : 1)) != 0;
  if (selector == @selector(setAccessibilityValue:))
    return self.isAccessibilityEnabled &&
           (state.data.actions & (state.data.scroll_orientation ? 128 : 4)) &&
           !(state.data.flags & SWAXReadOnly);
  if (selector == @selector(accessibilityMinValue) ||
      selector == @selector(accessibilityMaxValue) ||
      selector == @selector(accessibilityOrientation))
    return self.isAccessibilityElement && state.data.scroll_orientation;
  if (selector == @selector(accessibilityHorizontalScrollBar))
    return self.accessibilityHorizontalScrollBar != nil;
  if (selector == @selector(accessibilityVerticalScrollBar))
    return self.accessibilityVerticalScrollBar != nil;
  if (selector == @selector(accessibilityPerformDecrement) ||
      selector == @selector(accessibilityPerformIncrement))
    return self.isAccessibilityEnabled && state.data.scroll_orientation &&
           !(state.data.flags & SWAXReadOnly) &&
           (state.data.actions &
            (selector == @selector(accessibilityPerformDecrement) ? 256 : 512));
  if (selector == @selector(setAccessibilityFocused:))
    return self.isAccessibilityEnabled && (state.data.actions & 8);
  if (selector == @selector(accessibilityPerformShowMenu))
    return self.isAccessibilityEnabled && (state.data.actions & 64);
  if (selector == @selector(accessibilityFrame))
    return self.number == 0 || state.data.geometry;
  if (selector == @selector(accessibilityActivationPoint))
    return NO;
  if (selector == @selector(accessibilityHitTest:))
    return [self participatesInAccessibility];
  if (selector == @selector(accessibilityNumberOfCharacters) ||
      selector == @selector(accessibilityStringForRange:) ||
      selector == @selector(accessibilityAttributedStringForRange:))
    return self.accessibleText != nil;
  if (selector == @selector(accessibilitySelectedText) ||
      selector == @selector(accessibilitySelectedTextRange) ||
      selector == @selector(accessibilitySelectedTextRanges))
    return self.accessibleText && state.selection.location != NSNotFound;
  if (selector == @selector(setAccessibilitySelectedTextRange:) ||
      selector == @selector(setAccessibilitySelectedTextRanges:))
    return self.isAccessibilityEnabled && self.accessibleText &&
           (state.data.actions & 16);
  if (selector == @selector(setAccessibilitySelectedText:))
    return self.isAccessibilityEnabled && self.accessibleText &&
           state.selection.location != NSNotFound &&
           !(state.data.flags & SWAXReadOnly) && (state.data.actions & 32);
  NSString *name = NSStringFromSelector(selector);
  if ([name hasPrefix:@"accessibilityPerform"] ||
      [name hasPrefix:@"setAccessibility"])
    return NO;
  return [super isAccessibilitySelectorAllowed:selector];
}
@end
