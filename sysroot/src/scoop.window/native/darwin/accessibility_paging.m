#import "accessibility_collection.h"

BOOL sw_accessibility_list_needs_pages(SWAccessibilityState *state) {
  return state && state.data.role == 12 && state.itemCount > 4096;
}

@interface SWAccessibilityRowNavigator : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityListPageGroup *group;
@end

@interface SWAccessibilityListPageGroup ()
@property(nonatomic, strong) SWAccessibilityRowNavigator *navigator;
@property(nonatomic) NSUInteger start, row;
@property(nonatomic) NSRange viewport;
@property(nonatomic) BOOL pageChosen;
@property(nonatomic, copy) NSDictionary<id, NSNumber *> *visibleIndices;
@property(nonatomic, copy) id focusedKey;
@property(nonatomic) uint64_t revision;
- (SWAccessibilityNode *)list;
- (void)goToRow:(NSUInteger)row;
@end

@implementation SWAccessibilityListPageGroup
- (instancetype)init {
  if ((self = [super init])) {
    _navigator = [SWAccessibilityRowNavigator new];
    _navigator.group = self;
  }
  return self;
}
- (SWAccessibilityNode *)list {
  return [self.collection.tree element:self.collection.number];
}
- (NSRange)range {
  NSUInteger total = (NSUInteger)MAX(self.collection.state.itemCount, 0);
  NSUInteger start = MIN(self.start, total);
  return NSMakeRange(start, MIN(SWAccessibilityListPageSize, total - start));
}
- (void)setStart:(NSUInteger)start row:(NSUInteger)row {
  self.pageChosen = YES;
  BOOL moved = self.start != start;
  BOOL changed = self.row != row;
  NSUInteger oldLength = self.range.length;
  self.start = start;
  self.row = row;
  if (moved) {
    NSAccessibilityPostNotification(self,
                                    NSAccessibilityTitleChangedNotification);
    if (!self.collection.tree.pending)
      NSAccessibilityPostNotification(self.list,
                                      NSAccessibilityLayoutChangedNotification);
    if (self.range.length != oldLength)
      NSAccessibilityPostNotification(
          self.list, NSAccessibilityRowCountChangedNotification);
  }
  if (changed)
    NSAccessibilityPostNotification(self.navigator,
                                    NSAccessibilityValueChangedNotification);
}
- (void)goToRow:(NSUInteger)row {
  if (row < (NSUInteger)MAX(self.collection.state.itemCount, 0))
    [self setStart:row - row % SWAccessibilityListPageSize row:row];
}
- (void)updateViewport {
  SWAccessibilityCollection *collection = self.collection;
  SWAccessibilityTree *tree = collection.tree;
  NSMutableDictionary *indices = [NSMutableDictionary new];
  NSInteger first = NSNotFound, last = -1;
  BOOL sameSnapshot = self.revision == collection.state.collectionRevision;
  for (NSNumber *number in tree.children[@(collection.number)]) {
    SWAccessibilityState *state = tree.published[number];
    if (!state.itemKey || !state.data.geometry || !state.data.visible ||
        (state.data.flags & SWAXHidden) ||
        [tree.retiring containsObject:number])
      continue;
    NSNumber *known = sameSnapshot ? self.visibleIndices[state.itemKey] : nil;
    NSInteger index = known ? known.integerValue
                            : [collection publishedIndexForKey:state.itemKey];
    if (index != NSNotFound) {
      indices[state.itemKey] = @(index);
      first = MIN(first, index);
      last = MAX(last, index);
    }
  }
  BOOL scrolled = NO;
  if (last >= 0) {
    NSRange viewport =
        NSMakeRange((NSUInteger)first, (NSUInteger)(last - first) + 1);
    // The first frame can arrive after an AX client has already chosen a page.
    scrolled = self.viewport.length ? !NSEqualRanges(self.viewport, viewport)
                                    : !self.pageChosen;
    self.viewport = viewport;
  }
  self.visibleIndices = indices;
  self.revision = collection.state.collectionRevision;

  id focus = nil;
  for (SWAccessibilityState *state = tree.published[@(tree.focused)];
       state && state.data.parent;
       state = tree.published[@(state.data.parent)]) {
    if (state.data.parent == collection.number) {
      focus = state.itemKey;
      break;
    }
  }
  BOOL focusChanged = focus && ![focus isEqual:self.focusedKey];
  self.focusedKey = focus;
  if (focusChanged) {
    NSNumber *visible = indices[focus];
    NSInteger index = visible ? visible.integerValue
                              : [collection publishedIndexForKey:focus];
    if (index != NSNotFound) {
      [self goToRow:(NSUInteger)index];
      return;
    }
  }
  NSUInteger total = (NSUInteger)collection.state.itemCount;
  if (self.row >= total)
    [self goToRow:total - 1];
  NSRange range = self.range;
  if (scrolled && (!NSLocationInRange((NSUInteger)first, range) ||
                   !NSLocationInRange((NSUInteger)last, range))) {
    NSUInteger start =
        (NSUInteger)first - (NSUInteger)first % SWAccessibilityListPageSize;
    // A viewport can straddle two aligned pages. Keep its visible rows
    // together.
    if ((NSUInteger)last - start >= SWAccessibilityListPageSize)
      start = MIN((NSUInteger)first, total - SWAccessibilityListPageSize);
    [self setStart:start row:(NSUInteger)first];
  }
}
- (BOOL)containsVisibleKey:(id)key {
  NSNumber *index = self.visibleIndices[key];
  return index && NSLocationInRange(index.unsignedIntegerValue, self.range);
}
- (BOOL)isAccessibilityElement {
  return self.list.isAccessibilityElement;
}
- (NSAccessibilityRole)accessibilityRole {
  return self.isAccessibilityElement ? NSAccessibilityGroupRole : nil;
}
- (NSString *)accessibilityLabel {
  NSRange range = self.range;
  return range.length
             ? [NSString stringWithFormat:@"Rows %lu–%lu of %lld",
                                          range.location + 1, NSMaxRange(range),
                                          self.collection.state.itemCount]
             : nil;
}
- (id)accessibilityParent {
  return self.list.publishedAccessibilityParent;
}
- (id)accessibilityWindow {
  return self.list.accessibilityWindow;
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (NSRect)accessibilityFrame {
  return self.list.accessibilityFrame;
}
- (NSArray *)accessibilityChildren {
  return self.isAccessibilityElement ? @[ self.list, self.navigator ] : @[];
}
- (NSArray *)accessibilityChildrenInNavigationOrder {
  return self.accessibilityChildren;
}
- (void)retire {
  self.collection = nil;
  self.navigator.group = nil;
  NSAccessibilityPostNotification(
      self.navigator, NSAccessibilityUIElementDestroyedNotification);
  NSAccessibilityPostNotification(
      self, NSAccessibilityUIElementDestroyedNotification);
}
@end

@implementation SWAccessibilityRowNavigator
- (BOOL)isAccessibilityElement {
  return self.group.isAccessibilityElement;
}
- (BOOL)isAccessibilityEnabled {
  return self.group.list.isAccessibilityEnabled;
}
- (NSAccessibilityRole)accessibilityRole {
  return self.isAccessibilityElement ? NSAccessibilityIncrementorRole : nil;
}
- (NSString *)accessibilityLabel {
  return self.isAccessibilityElement ? @"Go to row" : nil;
}
- (NSString *)accessibilityHelp {
  return self.isAccessibilityElement ? @"Enter a row number, or increment and "
                                       @"decrement to move between pages."
                                     : nil;
}
- (id)accessibilityParent {
  return self.group;
}
- (id)accessibilityWindow {
  return self.group.accessibilityWindow;
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (id)accessibilityValue {
  return self.isAccessibilityElement ? @(self.group.row + 1) : nil;
}
- (id)accessibilityMinValue {
  return self.isAccessibilityElement ? @1 : nil;
}
- (id)accessibilityMaxValue {
  return self.isAccessibilityElement ? @(self.group.collection.state.itemCount)
                                     : nil;
}
- (void)setAccessibilityValue:(id)value {
  if (!self.isAccessibilityEnabled || ![value isKindOfClass:NSNumber.class])
    return;
  int64_t row = [value longLongValue];
  if (row >= 1 && row <= self.group.collection.state.itemCount &&
      [value compare:@(row)] == NSOrderedSame)
    [self.group goToRow:(NSUInteger)(row - 1)];
}
- (BOOL)accessibilityPerformIncrement {
  NSUInteger next = NSMaxRange(self.group.range);
  if (!self.isAccessibilityEnabled ||
      next >= (NSUInteger)self.group.collection.state.itemCount)
    return NO;
  [self.group setStart:next row:next];
  return YES;
}
- (BOOL)accessibilityPerformDecrement {
  NSUInteger start = self.group.range.location;
  if (!self.isAccessibilityEnabled || !start)
    return NO;
  NSUInteger previous = start - MIN(start, SWAccessibilityListPageSize);
  [self.group setStart:previous row:previous];
  return YES;
}
- (BOOL)firstPage {
  if (!self.isAccessibilityEnabled)
    return NO;
  [self.group goToRow:0];
  return YES;
}
- (BOOL)lastPage {
  if (!self.isAccessibilityEnabled)
    return NO;
  [self.group goToRow:(NSUInteger)(self.group.collection.state.itemCount - 1)];
  return YES;
}
- (NSArray<NSAccessibilityCustomAction *> *)accessibilityCustomActions {
  if (!self.isAccessibilityEnabled)
    return @[];
  return @[
    [[NSAccessibilityCustomAction alloc] initWithName:@"First Page"
                                               target:self
                                             selector:@selector(firstPage)],
    [[NSAccessibilityCustomAction alloc] initWithName:@"Last Page"
                                               target:self
                                             selector:@selector(lastPage)]
  ];
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  if (!self.isAccessibilityElement)
    return NO;
  if (selector == @selector(setAccessibilityValue:) ||
      selector == @selector(accessibilityPerformIncrement) ||
      selector == @selector(accessibilityPerformDecrement) ||
      selector == @selector(accessibilityCustomActions))
    return self.isAccessibilityEnabled;
  return [super isAccessibilitySelectorAllowed:selector];
}
@end
