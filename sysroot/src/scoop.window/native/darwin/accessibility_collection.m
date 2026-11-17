#import "accessibility_collection.h"
@implementation SWAccessibilityItemData
@end

@implementation SWAccessibilityColumnData
@end

@implementation SWAccessibilityTree (Collections)
- (SWAccessibilityCollection *)collection:(int64_t)number {
  SWAccessibilityState *state = self.published[@(number)];
  if (self.closed || !state || state.itemCount < 0 ||
      [self.retiring containsObject:@(number)])
    return nil;
  SWAccessibilityCollection *value = self.collections[@(number)];
  if (!value) {
    value = [SWAccessibilityCollection new];
    value.tree = self;
    value.number = number;
    if (!self.collections)
      self.collections = [NSMutableDictionary new];
    self.collections[@(number)] = value;
  }
  return value;
}
@end

@implementation SWAccessibilityCollection
- (instancetype)init {
  if ((self = [super init]))
    _items = [NSMapTable strongToWeakObjectsMapTable];
  return self;
}
- (SWAccessibilityState *)state {
  SWAccessibilityTree *tree = self.tree;
  if (![NSThread isMainThread] || tree.closed ||
      [tree.retiring containsObject:@(self.number)])
    return nil;
  SWAccessibilityState *state = tree.published[@(self.number)];
  return state.itemCount >= 0 ? state : nil;
}
- (int32_t)query:(int32_t)operation
           index:(int64_t)index
             key:(id)key
           reply:(SWAccessibilityItemData *)reply {
  SWAccessibilityTree *tree = self.tree;
  if (!self.state || !tree.collectionCallback)
    return 4;
  BOOL text = [key isKindOfClass:NSString.class];
  NSData *bytes = text ? [key dataUsingEncoding:NSUTF8StringEncoding] : nil;
  return tree.collectionCallback(tree.collectionContext, self.number, operation,
                                 index, text ? 1 : 0,
                                 text ? 0 : [key longLongValue], bytes.bytes,
                                 (int64_t)bytes.length, (__bridge void *)reply);
}
- (id)itemForKey:(id)key {
  if (!key || !self.state)
    return nil;
  SWAccessibilityVirtualItem *item = [self.items objectForKey:key];
  if (!item || item.retired) {
    item = [SWAccessibilityVirtualItem new];
    item.key = key;
    item.collection = self;
    [self.items setObject:item forKey:key];
  }
  return item;
}
- (id)cachedItemForKey:(id)key {
  SWAccessibilityVirtualItem *item = [self.items objectForKey:key];
  return item.retired ? nil : item;
}
- (NSArray *)rowsFrom:(NSUInteger)index count:(NSUInteger)count {
  SWAccessibilityState *state = self.state;
  if (!state || index >= (uint64_t)state.itemCount ||
      ![self.tree element:self.number].isAccessibilityElement)
    return @[];
  NSUInteger length = MIN(count, (NSUInteger)state.itemCount - index);
  NSMutableArray *result = [NSMutableArray arrayWithCapacity:length];
  for (NSUInteger offset = 0; offset < length; ++offset) {
    SWAccessibilityItemData *data = [SWAccessibilityItemData new];
    int32_t status = [self query:0
                           index:(int64_t)(index + offset)
                             key:nil
                           reply:data];
    if (status != 1 || !data.key)
      return nil;
    SWAccessibilityVirtualItem *item = [self itemForKey:data.key];
    item.cached = data;
    item.revision = state.collectionRevision;
    [result addObject:item];
  }
  return result;
}
- (SWAccessibilityNode *)mountedRow:(id)key {
  for (NSNumber *number in self.tree.children[@(self.number)]) {
    SWAccessibilityNode *node = [self.tree element:number.longLongValue];
    if ([node.state.itemKey isEqual:key])
      return node;
  }
  return nil;
}
- (NSArray *)visibleRows {
  if (![self.tree element:self.number].isAccessibilityElement)
    return @[];
  NSMutableArray *result = [NSMutableArray new];
  for (NSNumber *number in self.tree.children[@(self.number)]) {
    SWAccessibilityState *state = self.tree.published[number];
    if (state.itemKey && state.data.geometry && state.data.visible &&
        !(state.data.flags & SWAXHidden) &&
        ![self.tree.retiring containsObject:number]) {
      id item = [self itemForKey:state.itemKey];
      if (item)
        [result addObject:item];
    }
  }
  return result;
}
- (void)refresh {
  // Only client-held proxies are visited; an ordinary update never scans count.
  for (SWAccessibilityVirtualItem *item in self.items.objectEnumerator
           .allObjects) {
    if ([self query:2 index:0 key:item.key reply:nil] == 2) {
      item.retired = YES;
      item.cached = nil;
      [self.items removeObjectForKey:item.key];
      NSAccessibilityPostNotification(
          item, NSAccessibilityUIElementDestroyedNotification);
    }
    [item refreshCells];
  }
}
- (void)retire {
  for (SWAccessibilityVirtualItem *item in self.items.objectEnumerator) {
    item.retired = YES;
    item.cached = nil;
    [item refreshCells];
  }
  [self.items removeAllObjects];
  self.tree = nil;
}
@end

@implementation SWAccessibilityVirtualItem
- (SWAccessibilityItemData *)itemData {
  if (self.retired)
    return nil;
  SWAccessibilityState *state = self.collection.state;
  if (!state)
    return nil;
  if (self.cached && self.revision == state.collectionRevision)
    return self.cached;
  SWAccessibilityItemData *data = [SWAccessibilityItemData new];
  int32_t status = [self.collection query:1 index:0 key:self.key reply:data];
  if (status == 2) {
    self.retired = YES;
    self.cached = nil;
  }
  if (status != 1)
    return nil;
  self.cached = data;
  self.revision = state.collectionRevision;
  return data;
}
- (BOOL)isAccessibilityElement {
  return !self.retired && self.collection.state &&
         [self.collection.tree element:self.collection.number]
             .isAccessibilityElement;
}
- (NSAccessibilityRole)accessibilityRole {
  return self.isAccessibilityElement ? NSAccessibilityRowRole : nil;
}
- (id)accessibilityParent {
  return self.isAccessibilityElement
             ? [self.collection.tree element:self.collection.number]
             : nil;
}
- (id)accessibilityWindow {
  return [self.accessibilityParent accessibilityWindow];
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (NSString *)accessibilityLabel {
  return self.itemData.name;
}
- (NSString *)accessibilityTitle {
  return self.itemData.name;
}
- (NSString *)accessibilityHelp {
  return self.itemData.help;
}
- (NSString *)accessibilityIdentifier {
  return self.itemData.identifier;
}
- (id)accessibilityValue {
  return self.itemData.value;
}
- (NSInteger)accessibilityIndex {
  SWAccessibilityItemData *data = self.itemData;
  return data ? (NSInteger)data.index : NSNotFound;
}
- (BOOL)isAccessibilityEnabled {
  return self.itemData.enabled &&
         [self.accessibilityParent isAccessibilityEnabled];
}
- (BOOL)isAccessibilitySelected {
  return self.itemData.selected == 1;
}
- (BOOL)isAccessibilityExpanded {
  return self.itemData.expanded == 1;
}
- (NSArray *)accessibilityChildren {
  if (!self.isAccessibilityElement)
    return @[];
  if (self.collection.state.gridColumns.count)
    return self.gridCells;
  SWAccessibilityNode *row = [self.collection mountedRow:self.key];
  return row ? row.accessibilityChildren : @[];
}
- (NSArray *)accessibilityChildrenInNavigationOrder {
  return self.accessibilityChildren;
}
- (NSRect)accessibilityFrame {
  return self.isAccessibilityElement
             ? [self.collection mountedRow:self.key].accessibilityFrame
             : NSZeroRect;
}
- (id)accessibilityHitTest:(NSPoint)point {
  return self.isAccessibilityElement ? [[self.collection mountedRow:self.key]
                                           accessibilityHitTest:point]
                                     : nil;
}
- (BOOL)performItem:(int32_t)operation {
  if (!self.isAccessibilityEnabled)
    return NO;
  int32_t flag = operation == 3   ? 1
                 : operation == 4 ? 2
                 : operation <= 6 ? 4
                                  : 8;
  if (!(self.collection.state.collectionActions & flag) ||
      ((flag == 4) && !self.itemData.focusable))
    return NO;
  return [self.collection query:operation index:0 key:self.key
                          reply:nil] == 1 &&
         self.isAccessibilityElement;
}
- (BOOL)realizeAccessibilityItem {
  return [self performItem:3];
}
- (BOOL)revealAccessibilityItem {
  return [self performItem:4];
}
- (NSArray<NSAccessibilityCustomAction *> *)accessibilityCustomActions {
  if (!self.isAccessibilityEnabled)
    return @[];
  int32_t flags = self.collection.state.collectionActions;
  NSMutableArray *actions = [NSMutableArray new];
  if (flags & 1)
    [actions addObject:[[NSAccessibilityCustomAction alloc]
                           initWithName:@"Realize Item"
                                 target:self
                               selector:@selector(realizeAccessibilityItem)]];
  if (flags & 2) {
    NSString *name = @"Scroll to Visible";
    if (@available(macOS 26.0, *)) {
      NSString *localized = NSAccessibilityActionDescription(
          NSAccessibilityScrollToVisibleAction);
      if (localized.length)
        name = localized;
    }
    [actions addObject:[[NSAccessibilityCustomAction alloc]
                           initWithName:name
                                 target:self
                               selector:@selector(revealAccessibilityItem)]];
  }
  return actions;
}
- (void)setAccessibilityFocused:(BOOL)focused {
  [self performItem:focused ? 5 : 6];
}
- (void)setAccessibilitySelected:(BOOL)selected {
  [self performItem:selected ? 7 : 8];
}
- (BOOL)isAccessibilityFocused {
  if (!self.isAccessibilityElement ||
      !self.collection.tree.view.window.isKeyWindow)
    return NO;
  SWAccessibilityNode *row = [self.collection mountedRow:self.key];
  int64_t number = self.collection.tree.focused;
  while (number && row) {
    if (number == row.number)
      return YES;
    number = self.collection.tree.published[@(number)].data.parent;
  }
  return NO;
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  if (!self.isAccessibilityElement)
    return NO;
  if (selector == @selector(isAccessibilitySelected))
    return self.itemData && self.itemData.selected >= 0;
  if (selector == @selector(isAccessibilityExpanded))
    return self.itemData && self.itemData.expanded >= 0;
  if (selector == @selector(accessibilityFrame))
    return [self.collection mountedRow:self.key].state.data.geometry != 0;
  if (selector == @selector(setAccessibilityFocused:) ||
      selector == @selector(isAccessibilityFocused))
    return self.itemData.focusable &&
           (self.collection.state.collectionActions & 4);
  if (selector == @selector(accessibilityCustomActions))
    return (self.collection.state.collectionActions & 3) != 0;
  if (selector == @selector(setAccessibilitySelected:))
    return self.collection.state.selectionKind >= 0 &&
           (self.collection.state.collectionActions & 8);
  if (selector == @selector(setAccessibilityExpanded:) ||
      selector == @selector(setAccessibilityValue:))
    return NO;
  return [super isAccessibilitySelectorAllowed:selector];
}
@end

static int64_t
collectionOwner(NSDictionary<NSNumber *, SWAccessibilityState *> *states,
                NSNumber *number) {
  SWAccessibilityState *value = states[number];
  while (value && value.data.parent) {
    int64_t parent = value.data.parent;
    value = states[@(parent)];
    if (value && value.itemCount >= 0)
      return parent;
  }
  return 0;
}

BOOL sw_accessibility_collections_commit(
    SWAccessibilityTree *tree,
    NSDictionary<NSNumber *, SWAccessibilityState *> *previous,
    NSDictionary<NSNumber *, SWAccessibilityState *> *changes,
    NSSet<NSNumber *> *removed) {
  BOOL rootChanged = NO;
  NSMutableSet<NSNumber *> *layout = [NSMutableSet new];
  NSMutableSet<NSNumber *> *visible = [NSMutableSet new];
  for (NSNumber *number in removed) {
    int64_t owner = collectionOwner(previous, number);
    if (owner) {
      [layout addObject:@(owner)];
      if (previous[number].itemKey && previous[number].data.visible)
        [visible addObject:@(owner)];
    } else {
      rootChanged = YES;
    }
    [tree.collections[number] retire];
    [tree.collections removeObjectForKey:number];
  }
  for (NSNumber *number in changes) {
    SWAccessibilityState *value = changes[number];
    SWAccessibilityState *old = previous[number];
    BOOL structure =
        !old || old.data.parent != value.data.parent ||
        old.data.order != value.data.order ||
        ((old.data.flags ^ value.data.flags) & (SWAXHidden | SWAXIgnored));
    int64_t owner = collectionOwner(tree.published, number);
    if (structure) {
      if (owner)
        [layout addObject:@(owner)];
      else
        rootChanged = YES;
      if (old && old.data.parent != value.data.parent) {
        int64_t before = collectionOwner(previous, number);
        if (before)
          [layout addObject:@(before)];
        else
          rootChanged = YES;
      }
    }
    if (owner && value.itemKey &&
        (!old || old.data.visible != value.data.visible ||
         old.data.geometry != value.data.geometry))
      [visible addObject:@(owner)];
    if (value.itemCount < 0) {
      [tree.collections[number] retire];
      [tree.collections removeObjectForKey:number];
    } else if (!old || value.collectionRevision != old.collectionRevision) {
      [tree.collections[number] refresh];
      [layout addObject:number];
      if (!old || value.itemCount != old.itemCount)
        NSAccessibilityPostNotification(
            [tree element:number.longLongValue],
            NSAccessibilityRowCountChangedNotification);
    }
  }
  // AppKit exposes visibleRows changes through the collection's layout
  // notification.
  [layout unionSet:visible];
  for (NSNumber *number in layout) {
    id node = [tree element:number.longLongValue];
    if (node)
      NSAccessibilityPostNotification(node,
                                      NSAccessibilityLayoutChangedNotification);
  }
  return rootChanged;
}

static id itemKey(int32_t kind, int64_t number, const uint8_t *text,
                  int64_t length) {
  if (kind == 0)
    return @(number);
  if (kind != 1 || length < 0 || (length && !text))
    return nil;
  return [[NSString alloc] initWithBytes:text
                                  length:(NSUInteger)length
                                encoding:NSUTF8StringEncoding];
}

void scoop_window_accessibility_collection_callback(
    void *pointer, SWAccessibilityCollectionCallback callback, void *context) {
  SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
  tree.collectionCallback = callback;
  tree.collectionContext = context;
}

int64_t scoop_window_accessibility_collection(void *pointer, int64_t number,
                                              int64_t count, uint64_t revision,
                                              int32_t actions) {
  SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
  SWAccessibilityState *state = tree.pending[@(number)];
  if (!state || count < -1 ||
      (count >= 0 && state.data.role != 12 && state.data.role != 22))
    return -4;
  state.itemCount = count;
  state.collectionRevision = revision;
  state.collectionActions = actions;
  return 0;
}

int64_t scoop_window_accessibility_collection_selection(void *pointer,
                                                        int64_t owner,
                                                        int32_t kind,
                                                        int32_t required) {
  SWAccessibilityState *state =
      ((__bridge SWAccessibilityTree *)pointer).pending[@(owner)];
  if (!state || (kind != 0 && kind != 1))
    return -4;
  state.selectionKind = kind;
  state.selectionRequired = required != 0;
  return 0;
}

int64_t scoop_window_accessibility_collection_selected(
    void *pointer, int64_t owner, int32_t kind, int64_t number,
    const uint8_t *text, int64_t length) {
  SWAccessibilityState *state =
      ((__bridge SWAccessibilityTree *)pointer).pending[@(owner)];
  id key = itemKey(kind, number, text, length);
  if (!state || state.selectionKind < 0 || !key)
    return -4;
  [state.selectedKeys addObject:key];
  return 0;
}

int64_t scoop_window_accessibility_collection_column(
    void *pointer, int64_t owner, int64_t index, int32_t kind, int64_t number,
    const uint8_t *text, int64_t length, int64_t header) {
  SWAccessibilityState *state =
      ((__bridge SWAccessibilityTree *)pointer).pending[@(owner)];
  id key = itemKey(kind, number, text, length);
  if (!state || state.data.role != 22 ||
      index != (int64_t)state.gridColumns.count || !key)
    return -4;
  SWAccessibilityColumnData *column = [SWAccessibilityColumnData new];
  column.key = key;
  column.header = header;
  [state.gridColumns addObject:column];
  return 0;
}

int64_t scoop_window_accessibility_collection_column_name(void *pointer,
                                                          int64_t owner,
                                                          int64_t index,
                                                          const uint8_t *text,
                                                          int64_t length) {
  SWAccessibilityState *state =
      ((__bridge SWAccessibilityTree *)pointer).pending[@(owner)];
  if (!state || index < 0 || index >= (int64_t)state.gridColumns.count ||
      length < 0)
    return -4;
  NSString *name = [[NSString alloc] initWithBytes:text
                                            length:(NSUInteger)length
                                          encoding:NSUTF8StringEncoding];
  if (!name)
    return -4;
  state.gridColumns[(NSUInteger)index].name = name;
  return 0;
}

int64_t scoop_window_accessibility_collection_cell(void *pointer, int64_t owner,
                                                   int32_t kind, int64_t number,
                                                   const uint8_t *text,
                                                   int64_t length) {
  SWAccessibilityState *state =
      ((__bridge SWAccessibilityTree *)pointer).pending[@(owner)];
  id key = itemKey(kind, number, text, length);
  if (!state || !key)
    return -4;
  state.cellColumnKey = key;
  return 0;
}

int64_t scoop_window_accessibility_collection_row(void *pointer, int64_t node,
                                                  int32_t kind, int64_t number,
                                                  const uint8_t *text,
                                                  int64_t length) {
  SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
  SWAccessibilityState *state = tree.pending[@(node)];
  if (!state)
    return -4;
  state.itemKey = itemKey(kind, number, text, length);
  return state.itemKey ? 0 : -4;
}

void scoop_window_accessibility_collection_item(void *pointer, int64_t index,
                                                int64_t count, int32_t enabled,
                                                int32_t selected,
                                                int32_t expanded,
                                                int32_t focusable) {
  SWAccessibilityItemData *data = (__bridge SWAccessibilityItemData *)pointer;
  data.index = index;
  data.count = count;
  data.enabled = enabled;
  data.selected = selected;
  data.expanded = expanded;
  data.focusable = focusable;
}
void scoop_window_accessibility_collection_item_key(void *pointer, int32_t kind,
                                                    int64_t number,
                                                    const uint8_t *text,
                                                    int64_t length) {
  SWAccessibilityItemData *data = (__bridge SWAccessibilityItemData *)pointer;
  data.key = itemKey(kind, number, text, length);
}
void scoop_window_accessibility_collection_item_text(void *pointer,
                                                     int32_t field,
                                                     const uint8_t *text,
                                                     int64_t length) {
  SWAccessibilityItemData *data = (__bridge SWAccessibilityItemData *)pointer;
  NSString *value = [[NSString alloc] initWithBytes:text
                                             length:(NSUInteger)length
                                           encoding:NSUTF8StringEncoding];
  switch (field) {
  case 0:
    data.name = value;
    break;
  case 1:
    data.help = value;
    break;
  case 2:
    data.identifier = value;
    break;
  case 3:
    data.value = value;
    break;
  }
}

int64_t scoop_window_accessibility_collection_active(
    void *pointer, int64_t number, int64_t collection, int32_t kind,
    int64_t key_number, const uint8_t *bytes, int64_t length) {
  @autoreleasepool {
    SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
    SWAccessibilityState *state = tree.pending[@(number)];
    if (!state || tree.closed)
      return -4;
    id key = kind == 0 ? @(key_number)
             : kind == 1 && length >= 0
                 ? [[NSString alloc] initWithBytes:bytes
                                            length:(NSUInteger)length
                                          encoding:NSUTF8StringEncoding]
                 : nil;
    if (!key)
      return 1;
    state.activeCollection = collection;
    state.activeKey = key;
    return 0;
  }
}

SWAccessibilityBytes
scoop_window_accessibility_collection_search(void *pointer) {
  NSData *search = ((__bridge SWAccessibilityItemData *)pointer).search;
  return (SWAccessibilityBytes){search.bytes,
                                search ? (int64_t)search.length : -1};
}
