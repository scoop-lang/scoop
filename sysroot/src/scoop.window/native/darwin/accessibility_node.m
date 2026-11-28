#import "accessibility_collection.h"
#include <math.h>

static BOOL isCollectionRowArray(NSAccessibilityAttributeName attribute) {
  return
      [attribute isEqual:NSAccessibilityChildrenAttribute] ||
      [attribute isEqual:NSAccessibilityChildrenInNavigationOrderAttribute] ||
      [attribute isEqual:NSAccessibilityRowsAttribute];
}

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
  if (state.data.role == 4 && state.data.expanded >= 0)
    return NSAccessibilityComboBoxRole;
  if (state.data.role == 4 && (state.data.flags & SWAXMultiline) &&
      !(state.data.flags & SWAXProtected))
    return NSAccessibilityTextAreaRole;
  if (state.data.role == 2 && state.data.expanded >= 0)
    return NSAccessibilityPopUpButtonRole;
  if (state.data.role == 14) {
    if (@available(macOS 26.0, *))
      return NSAccessibilityHeadingRole;
    return NSAccessibilityStaticTextRole;
  }
  NSAccessibilityRole roles[] = {NSAccessibilityGroupRole,
                                 NSAccessibilityStaticTextRole,
                                 NSAccessibilityButtonRole,
                                 NSAccessibilityCheckBoxRole,
                                 NSAccessibilityTextFieldRole,
                                 NSAccessibilityScrollAreaRole,
                                 NSAccessibilityScrollBarRole,
                                 NSAccessibilityButtonRole,
                                 NSAccessibilityButtonRole,
                                 NSAccessibilityRadioButtonRole,
                                 NSAccessibilityRadioGroupRole,
                                 NSAccessibilityImageRole,
                                 NSAccessibilityListRole,
                                 NSAccessibilityLinkRole,
                                 NSAccessibilityStaticTextRole,
                                 NSAccessibilitySliderRole,
                                 NSAccessibilityProgressIndicatorRole,
                                 NSAccessibilityMenuRole,
                                 NSAccessibilityMenuItemRole,
                                 NSAccessibilityTabGroupRole,
                                 NSAccessibilityRadioButtonRole,
                                 NSAccessibilityGroupRole,
                                 NSAccessibilityTableRole,
                                 NSAccessibilityCellRole,
                                 NSAccessibilityCellRole,
                                 NSAccessibilityCellRole,
                                 NSAccessibilityDisclosureTriangleRole,
                                 NSAccessibilitySplitterRole,
                                 NSAccessibilityRowRole,
                                 NSAccessibilityOutlineRole,
                                 NSAccessibilityRowRole};
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
  if (state.data.role == 20)
    return NSAccessibilityTabButtonSubrole;
  if (state.data.role == 21)
    return NSAccessibilityDialogSubrole;
  return nil;
}
- (BOOL)usesToggleAction {
  int32_t role = self.state.data.role;
  return role == 3 || role == 7 || role == 8 ||
         ((role == 18 || role == 30) && self.state.data.checked >= 0);
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
  SWAccessibilityState *state = self.state;
  NSMutableArray *result = [[self relatedElements:state.controls] mutableCopy];
  id active = state.activeNode ? [self.tree element:state.activeNode]
              : state.activeCollection
                  ? [[self.tree collection:state.activeCollection]
                        itemForKey:state.activeKey]
                  : nil;
  if ([active isAccessibilityElement] && ![result containsObject:active])
    [result addObject:active];
  return result;
}
- (NSArray *)accessibilityRowHeaderUIElements {
  return (self.state.data.role == 22 || self.state.data.role == 29)
             ? sw_accessibility_table_headers(self, YES)
             : [self relatedElements:self.state.rowHeaders];
}
- (NSArray *)accessibilityColumnHeaderUIElements {
  return (self.state.data.role == 22 || self.state.data.role == 29)
             ? sw_accessibility_table_headers(self, NO)
             : [self relatedElements:self.state.columnHeaders];
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
- (BOOL)isAccessibilityModal {
  return (self.state.data.flags & SWAXModal) != 0;
}
- (BOOL)isAccessibilitySelected {
  return self.state.data.selected == 1;
}
- (BOOL)isAccessibilityExpanded {
  return self.state.data.expanded == 1;
}
- (NSURL *)accessibilityURL {
  NSString *url = self.state.url;
  return url ? [NSURL URLWithString:url] : nil;
}
- (NSString *)accessibilityValueDescription {
  SWAccessibilityData data = self.state.data;
  if ((data.flags & SWAXInvalid) && (data.flags & SWAXBusy))
    return @"Invalid, busy";
  if (data.flags & SWAXInvalid)
    return @"Invalid";
  return (data.flags & SWAXBusy) ? @"Busy" : nil;
}
- (NSArray *)accessibilitySelectedChildren {
  NSMutableArray *selected = [NSMutableArray new];
  SWAccessibilityCollection *collection = [self.tree collection:self.number];
  if (collection) {
    if (collection.pageGroup) {
      if (self.state.selectedKeys.count == 0)
        return @[];
      NSRange range = collection.presentedRange;
      return [collection rowsFrom:range.location
                            count:range.length
                     selectedOnly:YES];
    }
    for (id key in self.state.selectedKeys) {
      id item = [collection itemForKey:key];
      if ([item isAccessibilityElement])
        [selected addObject:item];
    }
    return selected;
  }
  NSMutableArray *pending = [self.accessibilityChildren mutableCopy];
  while (pending.count) {
    id item = pending.lastObject;
    [pending removeLastObject];
    if (![item isKindOfClass:SWAccessibilityNode.class])
      continue;
    SWAccessibilityState *state = ((SWAccessibilityNode *)item).state;
    if (state.data.selected == 1 ||
        (state.data.role == 9 && state.data.checked == 1))
      [selected insertObject:item atIndex:0];
    else if (state.selectionKind < 0 && state.itemCount < 0)
      [pending addObjectsFromArray:[item accessibilityChildren]];
  }
  return selected;
}
- (NSArray *)accessibilityTabs {
  if (self.state.data.role != 19)
    return nil;
  NSMutableArray *tabs = [NSMutableArray new];
  for (id item in self.accessibilityChildren)
    if ([item isKindOfClass:SWAccessibilityNode.class] &&
        ((SWAccessibilityNode *)item).state.data.role == 20)
      [tabs addObject:item];
  return tabs;
}
- (id)accessibilityValue {
  SWAccessibilityState *state = self.state;
  if (!state || (state.data.flags & SWAXProtected))
    return nil;
  if (state.data.has_range)
    return @(state.data.number_value);
  if (state.data.role == 14)
    return state.data.heading_level ? @(state.data.heading_level) : nil;
  if (state.data.role == 20)
    return @(state.data.selected == 1);
  if (state.data.role == 6) {
    if (!(state.data.flags & SWAXHasScrollRange))
      return nil;
    double maximum =
        fmax(state.data.scroll_content - state.data.scroll_viewport, 0);
    return @(maximum > 0 ? state.data.scroll_offset / maximum : 0);
  }
  return state.data.checked >= 0 ? @(state.data.checked) : state.value;
}
- (id)accessibilityMinValue {
  SWAccessibilityData data = self.state.data;
  return data.has_range ? @(data.minimum) : (data.flags & SWAXHasScrollRange) ? @0.0 : nil;
}
- (id)accessibilityMaxValue {
  SWAccessibilityData data = self.state.data;
  if (data.has_range)
    return @(data.maximum);
  return (data.flags & SWAXHasScrollRange)
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
    if ((state.data.flags & SWAXHasScrollRange) &&
        state.data.scroll_orientation == orientation) {
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
  return self.tree.collections[@(self.number)].pageGroup
             ?: self.publishedAccessibilityParent;
}
- (id)publishedAccessibilityParent {
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
    id cell = sw_accessibility_virtual_cell(self.tree, parent);
    if (cell)
      return cell;
    if (ancestor.itemKey) {
      id item = [[self.tree collection:ancestor.data.parent]
          itemForKey:ancestor.itemKey];
      if (item)
        return item;
    }
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
- (void)appendChildrenOf:(int64_t)parent
                      to:(NSMutableArray *)children
             visibleOnly:(BOOL)visible {
  SWAccessibilityTree *tree = self.tree;
  for (NSNumber *number in tree.children[@(parent)]) {
    SWAccessibilityState *state = tree.published[number];
    if (!state || (state.data.flags & SWAXHidden) ||
        [tree.retiring containsObject:number])
      continue;
    if (state.data.flags & SWAXIgnored)
      [self appendChildrenOf:number.longLongValue
                          to:children
                 visibleOnly:visible];
    else if (!visible || (state.data.geometry && state.data.visible)) {
      SWAccessibilityNode *child = [tree element:number.longLongValue];
      if (child.isAccessibilityElement)
        [children addObject:tree.collections[number].pageGroup ?: (id)child];
    }
  }
}
- (NSArray *)accessibilityChildren {
  if (![self participatesInAccessibility])
    return @[];
  SWAccessibilityCollection *collection = [self.tree collection:self.number];
  if (collection)
    return [collection presentedRowsFrom:0 count:NSUIntegerMax];
  NSMutableArray *children = [NSMutableArray new];
  [self appendChildrenOf:self.number to:children visibleOnly:NO];
  return children;
}
- (NSArray *)accessibilityChildrenInNavigationOrder {
  return self.accessibilityChildren;
}
- (NSArray *)accessibilityVisibleChildren {
  SWAccessibilityCollection *collection = [self.tree collection:self.number];
  if (collection)
    return collection.visibleRows;
  NSMutableArray *visible = [NSMutableArray new];
  if ([self participatesInAccessibility])
    [self appendChildrenOf:self.number to:visible visibleOnly:YES];
  return visible;
}
- (NSInteger)accessibilityRowCount {
  if (self.state.itemCount >= 0)
    return (NSInteger)[self.tree collection:self.number].presentedRange.length;
  if ((self.state.data.role == 12 || self.state.data.role == 29) &&
      self.state.itemCount < 0)
    return (NSInteger)sw_accessibility_table_rows(self, NO).count;
  return MAX(self.state.data.table_rows, 0);
}
- (NSArray *)accessibilityRows {
  return sw_accessibility_table_rows(self, NO);
}
- (NSArray *)accessibilityVisibleRows {
  return sw_accessibility_table_rows(self, YES);
}
- (NSUInteger)accessibilityArrayAttributeCount:
    (NSAccessibilityAttributeName)attribute {
  if (self.state.itemCount >= 0 && isCollectionRowArray(attribute))
    return self.isAccessibilityElement ? (NSUInteger)self.accessibilityRowCount
                                       : 0;
  return [super accessibilityArrayAttributeCount:attribute];
}
- (NSArray *)accessibilityArrayAttributeValues:
                 (NSAccessibilityAttributeName)attribute
                                         index:(NSUInteger)index
                                      maxCount:(NSUInteger)count {
  if (self.state.itemCount >= 0 && isCollectionRowArray(attribute))
    return [[self.tree collection:self.number] presentedRowsFrom:index
                                                           count:count];
  return [super accessibilityArrayAttributeValues:attribute
                                            index:index
                                         maxCount:count];
}
- (NSUInteger)accessibilityIndexOfChild:(id)child {
  SWAccessibilityCollection *collection = [self.tree collection:self.number];
  if (!collection)
    return [super accessibilityIndexOfChild:child];
  if (!self.isAccessibilityElement ||
      ![child isKindOfClass:SWAccessibilityVirtualItem.class])
    return NSNotFound;
  SWAccessibilityVirtualItem *row = child;
  if (row.collection != collection)
    return NSNotFound;
  NSUInteger index = (NSUInteger)row.accessibilityIndex;
  NSRange range = collection.presentedRange;
  return NSLocationInRange(index, range) ? index - range.location : NSNotFound;
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
- (NSPoint)accessibilityActivationPoint {
  SWView *view = self.tree.view;
  SWAccessibilityData data = self.state.data;
  if (!data.activation || !self.isAccessibilityEnabled || !view.window)
    return NSZeroPoint;
  NSPoint point =
      [view convertPoint:NSMakePoint(data.activation_x, data.activation_y)
                  toView:nil];
  return [view.window convertPointToScreen:point];
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
    while (node.number && (node.state.data.flags & SWAXIgnored)) {
      if (node.state.itemKey) {
        id item = [[tree collection:node.state.data.parent]
            itemForKey:node.state.itemKey];
        if ([item isAccessibilityElement])
          return item;
      }
      node = [tree element:node.state.data.parent];
    }
    if (node.isAccessibilityElement)
      return sw_accessibility_virtual_cell(tree, node.number) ?: node;
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
  if (state.data.has_range) {
    if ((state.data.actions & 2048) && !(state.data.flags & SWAXReadOnly) &&
        [value isKindOfClass:NSNumber.class])
      [self perform:12
               text:nil
              range:NSMakeRange(NSNotFound, 0)
           position:[value doubleValue]];
    return;
  }
  if (state.data.role == 6) {
    if ((state.data.flags & SWAXHasScrollRange) && (state.data.actions & 128) &&
        !(state.data.flags & SWAXReadOnly) &&
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
  if (self.state.data.has_range)
    return (self.state.data.actions & 8192) && [self perform:14 text:nil] == 1;
  return (self.state.data.flags & SWAXHasScrollRange) &&
         (self.state.data.actions & 256) &&
         [self perform:9 text:nil] == 1;
}
- (BOOL)accessibilityPerformIncrement {
  if (self.state.data.has_range)
    return (self.state.data.actions & 4096) && [self perform:13 text:nil] == 1;
  return (self.state.data.flags & SWAXHasScrollRange) &&
         (self.state.data.actions & 512) &&
         [self perform:10 text:nil] == 1;
}
- (BOOL)accessibilityPerformShowMenu {
  if (!(self.state.data.actions & 64))
    return NO;
  return [self perform:7 text:nil] == 1;
}
- (void)setAccessibilitySelected:(BOOL)selected {
  if (self.state.data.selected >= 0 && (self.state.data.actions & 16384))
    [self perform:selected ? 15 : 16 text:nil];
}
- (void)setAccessibilityExpanded:(BOOL)expanded {
  if (self.state.data.expanded >= 0 && (self.state.data.actions & 32768))
    [self perform:expanded ? 17 : 18 text:nil];
}
- (BOOL)accessibilityPerformCancel {
  return (self.state.data.actions & 65536) && [self perform:19 text:nil] == 1;
}
- (BOOL)revealAccessibilityNode {
  if (!(self.state.data.actions & 1024))
    return NO;
  return [self perform:11 text:nil] == 1;
}
- (NSArray<NSAccessibilityCustomAction *> *)accessibilityCustomActions {
  if (!self.isAccessibilityEnabled || !(self.state.data.actions & 1024))
    return @[];
  NSString *name = @"Scroll to Visible";
  if (@available(macOS 26.0, *)) {
    NSString *localized =
        NSAccessibilityActionDescription(NSAccessibilityScrollToVisibleAction);
    if (localized.length)
      name = localized;
  }
  return @[ [[NSAccessibilityCustomAction alloc]
      initWithName:name
            target:self
          selector:@selector(revealAccessibilityNode)] ];
}
- (void)setAccessibilityFocused:(BOOL)focused {
  if (self.state.data.actions & 8)
    [self perform:focused ? 3 : 4 text:nil];
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  if (selector == @selector(accessibilityRows) ||
      selector == @selector(accessibilityVisibleRows) ||
      selector == @selector(accessibilityRowCount))
    return (self.state.itemCount >= 0 || self.state.data.role == 12 ||
            (self.state.data.role == 22 || self.state.data.role == 29)) &&
           self.isAccessibilityElement;
  SWAccessibilityState *state = self.state;
  if (!state)
    return NO;
  if (selector == @selector(accessibilityColumns) ||
      selector == @selector(accessibilityColumnCount) ||
      selector == @selector(accessibilityCellForColumn:row:) ||
      selector == @selector(accessibilitySelectedRows))
    return self.isAccessibilityElement &&
           (state.data.role == 22 || state.data.role == 29);
  if (selector == @selector(accessibilityIndex))
    return self.isAccessibilityElement &&
           (state.data.role == 28 || state.data.role == 30);
  if (selector == @selector(accessibilityRowIndexRange) ||
      selector == @selector(accessibilityColumnIndexRange))
    return self.isAccessibilityElement && state.data.cell_row >= 0;
  if (selector == @selector(accessibilitySelectedChildren))
    return self.isAccessibilityElement && state.selectionKind >= 0;
  if (selector == @selector(isAccessibilitySelected))
    return self.isAccessibilityElement && state.data.selected >= 0;
  if (selector == @selector(isAccessibilityExpanded))
    return self.isAccessibilityElement && state.data.expanded >= 0;
  if (selector == @selector(setAccessibilitySelected:))
    return self.isAccessibilityEnabled && state.data.selected >= 0 &&
           !(state.data.flags & SWAXReadOnly) && (state.data.actions & 16384);
  if (selector == @selector(setAccessibilityExpanded:))
    return self.isAccessibilityEnabled && state.data.expanded >= 0 &&
           !(state.data.flags & SWAXReadOnly) && (state.data.actions & 32768);
  if (selector == @selector(accessibilityPerformCancel))
    return self.isAccessibilityEnabled && (state.data.actions & 65536);
  if (selector == @selector(accessibilityTabs))
    return self.isAccessibilityElement && state.data.role == 19;
  if (selector == @selector(isAccessibilityRequired))
    return self.isAccessibilityElement;
  if (selector == @selector(accessibilityPerformPress))
    return self.isAccessibilityEnabled &&
           (state.data.actions & (self.usesToggleAction ? 3 : 1)) != 0;
  if (selector == @selector(setAccessibilityValue:))
    return self.isAccessibilityEnabled &&
           (state.data.role != 6 || (state.data.flags & SWAXHasScrollRange)) &&
           (state.data.actions & (state.data.has_range   ? 2048
                                  : state.data.role == 6 ? 128
                                                         : 4)) &&
           !(state.data.flags & SWAXReadOnly);
  if (selector == @selector(accessibilityMinValue) ||
      selector == @selector(accessibilityMaxValue))
    return self.isAccessibilityElement &&
           (state.data.has_range || (state.data.flags & SWAXHasScrollRange));
  if (selector == @selector(accessibilityOrientation))
    return self.isAccessibilityElement && state.data.scroll_orientation;
  if (selector == @selector(accessibilityHorizontalScrollBar))
    return self.accessibilityHorizontalScrollBar != nil;
  if (selector == @selector(accessibilityVerticalScrollBar))
    return self.accessibilityVerticalScrollBar != nil;
  if (selector == @selector(accessibilityPerformDecrement) ||
      selector == @selector(accessibilityPerformIncrement))
    return self.isAccessibilityEnabled &&
           (state.data.has_range || (state.data.flags & SWAXHasScrollRange)) &&
           !(state.data.flags & SWAXReadOnly) &&
           (state.data.actions &
            (state.data.has_range
                 ? (selector == @selector(accessibilityPerformDecrement) ? 8192
                                                                         : 4096)
                 : (selector == @selector(accessibilityPerformDecrement)
                        ? 256
                        : 512)));
  if (selector == @selector(setAccessibilityFocused:))
    return self.isAccessibilityEnabled && (state.data.actions & 8);
  if (selector == @selector(accessibilityPerformShowMenu))
    return self.isAccessibilityEnabled && (state.data.actions & 64);
  if (selector == @selector(accessibilityCustomActions))
    return self.isAccessibilityEnabled && (state.data.actions & 1024);
  if (selector == @selector(accessibilityFrame))
    return self.number == 0 || state.data.geometry;
  if (selector == @selector(accessibilityActivationPoint))
    return self.isAccessibilityEnabled && state.data.activation;
  if (selector == @selector(accessibilityHitTest:))
    return [self participatesInAccessibility];
  if (selector == @selector(accessibilityNumberOfCharacters) ||
      selector == @selector(accessibilityStringForRange:) ||
      selector == @selector(accessibilityAttributedStringForRange:))
    return self.accessibleText != nil;
  if (selector == @selector(accessibilityRangeForIndex:) ||
      selector == @selector(accessibilityRangeForPosition:) ||
      selector == @selector(accessibilityFrameForRange:) ||
      selector == @selector(accessibilityLineForIndex:) ||
      selector == @selector(accessibilityRangeForLine:) ||
      selector == @selector(accessibilityVisibleCharacterRange))
    return self.accessibleTextGeometry != nil;
  if (selector == @selector(accessibilityInsertionPointLineNumber))
    return self.accessibleTextGeometry &&
           state.selection.location != NSNotFound;
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
