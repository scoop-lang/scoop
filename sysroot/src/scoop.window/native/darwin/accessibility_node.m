#import "accessibility.h"

@implementation SWAccessibilityNode
- (SWAccessibilityState *)state {
  if (![NSThread isMainThread] || self.retired)
    return nil;
  SWAccessibilityTree *tree = self.tree;
  if (!tree || tree.closed || [tree.retiring containsObject:@(self.number)])
    return nil;
  return self.number == 0 ? tree.rootState : tree.published[@(self.number)];
}
- (BOOL)isAccessibilityElement {
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
- (NSAccessibilityRole)accessibilityRole {
  SWAccessibilityState *state = self.state;
  if (!state)
    return nil;
  NSAccessibilityRole roles[] = {
      NSAccessibilityGroupRole,     NSAccessibilityStaticTextRole,
      NSAccessibilityButtonRole,    NSAccessibilityCheckBoxRole,
      NSAccessibilityTextFieldRole, NSAccessibilityScrollAreaRole};
  return roles[state.data.role];
}
- (NSString *)accessibilitySubrole {
  return self.state.data.flags & SWAXProtected
             ? NSAccessibilitySecureTextFieldSubrole
             : nil;
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
- (id)accessibilityValue {
  SWAccessibilityState *state = self.state;
  if (!state || (state.data.flags & SWAXProtected))
    return nil;
  return state.data.checked >= 0 ? @(state.data.checked) : state.value;
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
  return self.number == 0 ? NSAccessibilityUnignoredAncestor(self.tree.view)
                          : [self.tree element:state.data.parent];
}
- (id)accessibilityWindow {
  return self.state ? self.tree.view.window : nil;
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (NSArray *)accessibilityChildren {
  if (!self.isAccessibilityElement)
    return @[];
  NSMutableArray<SWAccessibilityNode *> *children = [NSMutableArray new];
  SWAccessibilityTree *tree = self.tree;
  for (NSNumber *number in tree.published) {
    SWAccessibilityState *state = tree.published[number];
    if (state.data.parent == self.number) {
      SWAccessibilityNode *child = [tree element:number.longLongValue];
      if (child.isAccessibilityElement)
        [children addObject:child];
    }
  }
  [children sortUsingComparator:^NSComparisonResult(
                SWAccessibilityNode *left, SWAccessibilityNode *right) {
    int64_t a = left.state.data.order, b = right.state.data.order;
    if (a != b)
      return a < b ? NSOrderedAscending : NSOrderedDescending;
    return left.number < right.number ? NSOrderedAscending
                                      : NSOrderedDescending;
  }];
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
- (BOOL)isAccessibilityFocused {
  return self.state && self.number != 0 && self.tree.focused == self.number &&
         self.isAccessibilityElement && self.tree.view.window.keyWindow;
}
- (id)accessibilityFocusedUIElement {
  return self.state ? sw_accessibility_focused(self.tree.view) : nil;
}
- (int32_t)perform:(int32_t)action text:(NSString *)text {
  SWAccessibilityTree *tree = self.tree;
  if (!self.state || tree.closed || !tree.callback)
    return 4;
  if (!self.isAccessibilityEnabled)
    return 5;
  NSData *bytes = text ? [text dataUsingEncoding:NSUTF8StringEncoding] : nil;
  if (text && !bytes)
    return 2;
  SWAccessibilityCallback callback = tree.callback;
  void *context = tree.context;
  return callback(context, self.number, action, bytes.bytes,
                  text ? (int64_t)bytes.length : -1);
}
- (BOOL)accessibilityPerformPress {
  SWAccessibilityState *state = self.state;
  if (!state)
    return NO;
  int32_t action = state.data.role == 3 && (state.data.actions & 2) ? 1 : 0;
  if (!(state.data.actions & (1 << action)))
    return NO;
  return [self perform:action text:nil] == 1;
}
- (void)setAccessibilityValue:(id)value {
  SWAccessibilityState *state = self.state;
  if (!state || !(state.data.actions & 4) ||
      (state.data.flags & SWAXReadOnly) ||
      ![value isKindOfClass:NSString.class])
    return;
  [self perform:2 text:value];
}
- (void)setAccessibilityFocused:(BOOL)focused {
  if (self.state.data.actions & 8)
    [self perform:focused ? 3 : 4 text:nil];
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  SWAccessibilityState *state = self.state;
  if (!state)
    return NO;
  if (selector == @selector(accessibilityPerformPress))
    return self.isAccessibilityEnabled &&
           (state.data.actions & (state.data.role == 3 ? 3 : 1)) != 0;
  if (selector == @selector(setAccessibilityValue:))
    return self.isAccessibilityEnabled && (state.data.actions & 4) &&
           !(state.data.flags & SWAXReadOnly);
  if (selector == @selector(setAccessibilityFocused:))
    return self.isAccessibilityEnabled && (state.data.actions & 8);
  if (selector == @selector(accessibilityFrame))
    return self.number == 0 || state.data.geometry;
  if (selector == @selector(accessibilityActivationPoint))
    return NO;
  NSString *name = NSStringFromSelector(selector);
  if ([name hasPrefix:@"accessibilityPerform"] ||
      [name hasPrefix:@"setAccessibility"])
    return NO;
  return [super isAccessibilitySelectorAllowed:selector];
}
@end
