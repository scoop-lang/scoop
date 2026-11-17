#import "accessibility.h"

@implementation SWAccessibilityHitGeometry {
  SWAccessibilityHitData _value;
}
- (instancetype)initWithValue:(SWAccessibilityHitData)value {
  if ((self = [super init])) {
    _value = value;
    _value.retain(_value.context);
  }
  return self;
}
- (BOOL)containsPoint:(NSPoint)point {
  return _value.contains(_value.context, point.x, point.y) != 0;
}
- (void)dealloc {
  if (_value.context)
    _value.release(_value.context);
}
@end

@implementation SWAccessibilityState
- (void)setTextDocument:(SWTextDocument *)document
               revision:(uint64_t)revision
                  epoch:(uint64_t)epoch {
  sw_text_document_retain(document);
  sw_text_document_release(_textDocument);
  _textDocument = document;
  _documentRevision = revision;
  _documentEpoch = epoch;
}
- (void)dealloc {
  sw_text_document_release(_textDocument);
}
- (instancetype)init {
  if ((self = [super init]))
    _itemCount = -1;
  return self;
}
@end

@implementation SWAccessibilityTextGeometry
- (instancetype)initWithValue:(SWAccessibilityTextData)value {
  if ((self = [super init])) {
    _value = value;
    _value.retain(_value.context);
  }
  return self;
}
- (void)dealloc {
  if (_value.context)
    _value.release(_value.context);
}
@end

@implementation SWAccessibilityTree
- (SWAccessibilityNode *)element:(int64_t)number {
  if (self.closed)
    return nil;
  if (number == 0)
    return self.root;
  if (!self.published[@(number)] || [self.retiring containsObject:@(number)])
    return nil;
  SWAccessibilityNode *node = self.elements[@(number)];
  if (!node) {
    node = [SWAccessibilityNode new];
    node.number = number;
    node.tree = self;
    self.elements[@(number)] = node;
  }
  return node;
}
@end

id sw_accessibility_focused(SWView *view) {
  SWAccessibilityNode *root = (SWAccessibilityNode *)view.semanticRoot;
  SWAccessibilityTree *tree = root.tree;
  if (!tree || tree.closed || !tree.focused)
    return nil;
  SWAccessibilityNode *node = [tree element:tree.focused];
  return node.isAccessibilityElement ? node : nil;
}

SWResult scoop_window_accessibility_create(void *window,
                                           SWAccessibilityCallback callback,
                                           void *context) {
  @autoreleasepool {
    @try {
      SWView *view = ((__bridge SWWindow *)window).view;
      if (view.semanticRoot)
        return (SWResult){NULL, -5};
      SWAccessibilityTree *tree = [SWAccessibilityTree new];
      tree.view = view;
      tree.callback = callback;
      tree.context = context;
      tree.published = @{};
      tree.children = @{};
      tree.hitOrder = @[];
      tree.elements = [NSMutableDictionary new];
      tree.retiring = [NSMutableSet new];
      tree.rootState = [SWAccessibilityState new];
      tree.rootState.data =
          (SWAccessibilityData){.flags = SWAXEnabled, .checked = -1};
      tree.root = [SWAccessibilityNode new];
      tree.root.tree = tree;
      view.semanticRoot = tree.root;
      return (SWResult){(__bridge_retained void *)tree, 0};
    } @catch (NSException *exception) {
      return (SWResult){NULL, 1};
    }
  }
}

int64_t scoop_window_accessibility_begin(void *pointer) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      if (tree.closed || tree.pending)
        return -4;
      tree.pending = [NSMutableDictionary new];
      tree.removed = [NSMutableSet new];
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_accessibility_put(void *pointer,
                                       SWAccessibilityData data) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      if (!tree.pending || tree.closed)
        return -4;
      SWAccessibilityState *state = [SWAccessibilityState new];
      state.data = data;
      state.selection = NSMakeRange(NSNotFound, 0);
      state.labels = [NSMutableArray new];
      state.textChildren = [NSMutableArray new];
      state.controls = [NSMutableArray new];
      state.titleFor = [NSMutableArray new];
      state.descriptions = [NSMutableArray new];
      state.errors = [NSMutableArray new];
      state.rowHeaders = [NSMutableArray new];
      state.columnHeaders = [NSMutableArray new];
      state.itemCount = -1;
      state.selectionKind = -1;
      state.selectedKeys = [NSMutableArray new];
      state.gridColumns = [NSMutableArray new];
      tree.pending[@(data.number)] = state;
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_accessibility_text_child(void *pointer, int64_t number,
                                              int64_t child, int64_t start,
                                              int64_t end) {
  @autoreleasepool {
    @try {
      SWAccessibilityState *state =
          ((__bridge SWAccessibilityTree *)pointer).pending[@(number)];
      if (!state)
        return -4;
      [state.textChildren addObject:@{
        @"node" : @(child),
        @"start" : @(start),
        @"end" : @(end)
      }];
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_accessibility_document(void *pointer, int64_t number,
                                            void *document, uint64_t revision,
                                            uint64_t epoch) {
  SWAccessibilityState *state =
      ((__bridge SWAccessibilityTree *)pointer).pending[@(number)];
  if (!state)
    return -4;
  if (document) {
    SWTextDocumentState snapshot = sw_text_document_state(document);
    if (!snapshot.alive || !revision || snapshot.revision < revision ||
        (state.data.flags & SWAXProtected) || !state.value ||
        (snapshot.revision == revision &&
         snapshot.length !=
             (int64_t)[state.value
                 lengthOfBytesUsingEncoding:NSUTF8StringEncoding]))
      return -4;
  }
  [state setTextDocument:document revision:revision epoch:epoch];
  return 0;
}

int64_t scoop_window_accessibility_hit(void *pointer, int64_t number,
                                       SWAccessibilityHitData value) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      SWAccessibilityState *state = tree.pending[@(number)];
      if (!state || tree.closed)
        return -4;
      state.hit = [[SWAccessibilityHitGeometry alloc] initWithValue:value];
      return state.hit ? 0 : 1;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t
scoop_window_accessibility_text_geometry(void *pointer, int64_t number,
                                         SWAccessibilityTextData value) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      SWAccessibilityState *state = tree.pending[@(number)];
      if (!state || tree.closed)
        return -4;
      state.textGeometry =
          [[SWAccessibilityTextGeometry alloc] initWithValue:value];
      return state.textGeometry ? 0 : 1;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_accessibility_text(void *pointer, int64_t number,
                                        int32_t field, const uint8_t *bytes,
                                        int64_t length) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      SWAccessibilityState *state = tree.pending[@(number)];
      if (!state || tree.closed)
        return -4;
      NSString *value = length < 0 ? nil
                                   : [[NSString alloc]
                                         initWithBytes:bytes
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
      if (length >= 0 && !value)
        return 1;
      switch (field) {
      case 0:
        state.name = value;
        break;
      case 1:
        state.help = value;
        break;
      case 2:
        state.identifier = value;
        break;
      case 3:
        state.value = value;
        if (value && state.data.anchor >= 0 && state.data.cursor >= 0) {
          NSUInteger anchor = sw_text_utf16_offset(bytes, state.data.anchor);
          NSUInteger cursor = sw_text_utf16_offset(bytes, state.data.cursor);
          state.selection = NSMakeRange(
              MIN(anchor, cursor), MAX(anchor, cursor) - MIN(anchor, cursor));
        }
        break;
      case 4:
        state.error = value;
        break;
      case 5:
        state.url = value;
        break;
      default:
        return 1;
      }
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_accessibility_relation(void *pointer, int64_t number,
                                            int32_t field, int64_t target) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      SWAccessibilityState *state = tree.pending[@(number)];
      if (!state || tree.closed)
        return -4;
      switch (field) {
      case 0:
        [state.labels addObject:@(target)];
        break;
      case 1:
        [state.controls addObject:@(target)];
        break;
      case 2:
        [state.titleFor addObject:@(target)];
        break;
      case 3:
        [state.descriptions addObject:@(target)];
        break;
      case 4:
        [state.errors addObject:@(target)];
        break;
      case 5:
        [state.rowHeaders addObject:@(target)];
        break;
      case 6:
        [state.columnHeaders addObject:@(target)];
        break;
      case 7:
        state.activeNode = target;
        break;
      default:
        return 1;
      }
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_accessibility_remove(void *pointer, int64_t number) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      if (!tree.pending || tree.closed)
        return -4;
      [tree.removed addObject:@(number)];
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

static BOOL same(id left, id right) {
  return left == right || [left isEqual:right];
}

static NSDictionary<NSNumber *, NSArray<NSNumber *> *> *
childIndex(NSDictionary<NSNumber *, SWAccessibilityState *> *states) {
  NSMutableDictionary<NSNumber *, NSMutableArray<NSNumber *> *> *groups =
      [NSMutableDictionary new];
  for (NSNumber *number in states) {
    NSNumber *parent = @(states[number].data.parent);
    NSMutableArray *children = groups[parent];
    if (!children) {
      children = [NSMutableArray new];
      groups[parent] = children;
    }
    [children addObject:number];
  }
  NSMutableDictionary *result = [NSMutableDictionary new];
  for (NSNumber *parent in groups)
    result[parent] = [groups[parent]
        sortedArrayUsingComparator:^NSComparisonResult(NSNumber *left,
                                                       NSNumber *right) {
          int64_t a = states[left].data.order, b = states[right].data.order;
          if (a != b)
            return a < b ? NSOrderedAscending : NSOrderedDescending;
          return [left compare:right];
        }];
  return result;
}

static void notify(id node, SWAccessibilityState *old,
                   SWAccessibilityState *next) {
  if (!same(old.name, next.name))
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityTitleChangedNotification);
  if (old.data.scroll_orientation != next.data.scroll_orientation ||
      ((old.data.flags ^ next.data.flags) & SWAXRequired) ||
      old.data.actions != next.data.actions ||
      old.data.heading_level != next.data.heading_level ||
      old.data.expanded != next.data.expanded || !same(old.url, next.url) ||
      !same(old.help, next.help) || !same(old.error, next.error) ||
      ![old.labels isEqualToArray:next.labels] ||
      ![old.controls isEqualToArray:next.controls] ||
      ![old.titleFor isEqualToArray:next.titleFor])
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityLayoutChangedNotification);
  if (old.activeNode != next.activeNode ||
      old.activeCollection != next.activeCollection ||
      !same(old.activeKey, next.activeKey) ||
      ![old.rowHeaders isEqualToArray:next.rowHeaders] ||
      ![old.columnHeaders isEqualToArray:next.columnHeaders])
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityLayoutChangedNotification);
  if (!same(old.value, next.value) || old.data.checked != next.data.checked ||
      old.data.has_range != next.data.has_range ||
      old.data.minimum != next.data.minimum ||
      old.data.maximum != next.data.maximum ||
      old.data.number_value != next.data.number_value ||
      old.data.number_step != next.data.number_step ||
      old.data.scroll_offset != next.data.scroll_offset ||
      old.data.scroll_viewport != next.data.scroll_viewport ||
      old.data.scroll_content != next.data.scroll_content ||
      (old.data.flags & ~(SWAXRequired | SWAXCursorUpstream)) !=
          (next.data.flags & ~(SWAXRequired | SWAXCursorUpstream)))
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityValueChangedNotification);
  if (old.data.selected != next.data.selected ||
      ![old.selectedKeys isEqualToArray:next.selectedKeys])
    NSAccessibilityPostNotification(
        node, NSAccessibilitySelectedChildrenChangedNotification);
  if ((next.data.role == 1 || next.data.role == 4) &&
      !(next.data.flags & SWAXProtected) &&
      (!NSEqualRanges(old.selection, next.selection) ||
       (next.selection.location != NSNotFound &&
        (!same(old.value, next.value) || old.data.anchor != next.data.anchor ||
         old.data.cursor != next.data.cursor ||
         ((old.data.flags ^ next.data.flags) & SWAXCursorUpstream)))))
    NSAccessibilityPostNotification(
        node, NSAccessibilitySelectedTextChangedNotification);
  if (old.data.geometry != next.data.geometry ||
      old.data.visible != next.data.visible ||
      (old.textGeometry == nil) != (next.textGeometry == nil))
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityLayoutChangedNotification);
  else if (next.data.geometry) {
    if (old.data.x != next.data.x || old.data.y != next.data.y)
      NSAccessibilityPostNotification(node, NSAccessibilityMovedNotification);
    if (old.data.width != next.data.width ||
        old.data.height != next.data.height)
      NSAccessibilityPostNotification(node, NSAccessibilityResizedNotification);
  }
}

static NSArray<NSNumber *> *
hitIndex(NSDictionary<NSNumber *, SWAccessibilityState *> *states) {
  NSMutableArray<NSNumber *> *numbers = [NSMutableArray new];
  for (NSNumber *number in states)
    if (states[number].hit)
      [numbers addObject:number];
  return [numbers sortedArrayUsingComparator:^NSComparisonResult(
                      NSNumber *left, NSNumber *right) {
    int64_t a = states[left].data.scene_order;
    int64_t b = states[right].data.scene_order;
    return a < b   ? NSOrderedAscending
           : a > b ? NSOrderedDescending
                   : [left compare:right];
  }];
}

int64_t scoop_window_accessibility_commit(void *pointer, int64_t focused) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      if (!tree.pending || tree.closed)
        return -4;
      NSDictionary<NSNumber *, SWAccessibilityState *> *previous =
          tree.published;
      NSDictionary<NSNumber *, SWAccessibilityState *> *changes = tree.pending;
      NSSet<NSNumber *> *removed = tree.removed;
      NSMutableDictionary *next = [previous mutableCopy];
      for (NSNumber *number in removed)
        [next removeObjectForKey:number];
      [next addEntriesFromDictionary:changes];
      BOOL orderChanged = removed.count != 0;
      BOOL hitOrderChanged = removed.count != 0;
      for (NSNumber *number in changes) {
        SWAccessibilityState *old = previous[number];
        SWAccessibilityState *value = changes[number];
        SWAccessibilityData before = old ? old.data : (SWAccessibilityData){0};
        orderChanged |= !old || before.parent != value.data.parent ||
                        before.order != value.data.order;
        hitOrderChanged |= (old.hit == nil) != (value.hit == nil) ||
                           before.scene_order != value.data.scene_order;
      }
      NSDictionary *children = orderChanged ? childIndex(next) : tree.children;
      NSArray *hits = hitOrderChanged ? hitIndex(next) : tree.hitOrder;
      int64_t oldFocus = tree.focused;
      tree.published = next;
      tree.children = children;
      tree.hitOrder = hits;
      tree.focused = focused;
      BOOL rootChanged =
          sw_accessibility_collections_commit(tree, previous, changes, removed);
      sw_accessibility_grid_commit(tree);
      tree.pending = nil;
      tree.removed = nil;
      for (NSNumber *number in changes) {
        SWAccessibilityState *old = previous[number];
        SWAccessibilityState *value = changes[number];
        id target = value.itemKey ? [tree.collections[@(value.data.parent)]
                                        cachedItemForKey:value.itemKey]
                                  : [tree element:number.longLongValue];
        if (old && target)
          notify(target, old, value);
      }
      for (NSNumber *number in removed) {
        SWAccessibilityNode *node = tree.elements[number];
        if (node && !previous[number].itemKey)
          NSAccessibilityPostNotification(
              node, NSAccessibilityUIElementDestroyedNotification);
        [tree.elements removeObjectForKey:number];
        [tree.retiring removeObject:number];
      }
      if (rootChanged)
        NSAccessibilityPostNotification(
            tree.root, NSAccessibilityLayoutChangedNotification);
      if (oldFocus != focused)
        NSAccessibilityPostNotification(
            [tree element:focused] ?: tree.root,
            NSAccessibilityFocusedUIElementChangedNotification);
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

void scoop_window_accessibility_abort(void *pointer) {
  SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
  tree.pending = nil;
  tree.removed = nil;
}

void scoop_window_accessibility_retire(void *pointer, int64_t number) {
  SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
  [tree.retiring addObject:@(number)];
  tree.elements[@(number)].retired = YES;
}

void scoop_window_accessibility_close(void *pointer) {
  SWAccessibilityTree *tree = (__bridge_transfer SWAccessibilityTree *)pointer;
  tree.closed = YES;
  tree.callback = NULL;
  tree.context = NULL;
  tree.collectionCallback = NULL;
  tree.collectionContext = NULL;
  for (SWAccessibilityCollection *collection in tree.collections.allValues)
    [collection retire];
  [tree.collections removeAllObjects];
  tree.view.semanticRoot = nil;
  tree.view = nil;
  tree.root.retired = YES;
  for (SWAccessibilityNode *node in tree.elements.allValues)
    node.retired = YES;
  [tree.elements removeAllObjects];
  [tree.retiring removeAllObjects];
  tree.published = @{};
  tree.children = @{};
  tree.hitOrder = @[];
  tree.pending = nil;
  tree.removed = nil;
}

void scoop_window_accessibility_announce(void *pointer, int64_t number,
                                         const uint8_t *bytes, int64_t length,
                                         int32_t priority) {
  @autoreleasepool {
    SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
    SWAccessibilityNode *node = [tree element:number];
    if (![NSThread isMainThread] || tree.closed ||
        !node.isAccessibilityElement ||
        (node.state.data.flags & (SWAXProtected | SWAXBusy)) || length < 0)
      return;
    NSString *text = [[NSString alloc] initWithBytes:bytes
                                              length:(NSUInteger)length
                                            encoding:NSUTF8StringEncoding];
    if (!text)
      return;
    NSAccessibilityPostNotificationWithUserInfo(
        NSApp, NSAccessibilityAnnouncementRequestedNotification, @{
          NSAccessibilityAnnouncementKey : text,
          NSAccessibilityPriorityKey :
              @(priority == 2 ? NSAccessibilityPriorityHigh
                              : NSAccessibilityPriorityMedium)
        });
  }
}
