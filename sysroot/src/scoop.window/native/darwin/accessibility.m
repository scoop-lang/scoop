#import "accessibility.h"

@implementation SWAccessibilityState
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
      state.labels = [NSMutableArray new];
      state.controls = [NSMutableArray new];
      state.titleFor = [NSMutableArray new];
      tree.pending[@(data.number)] = state;
      return 0;
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
        break;
      case 4:
        state.error = value;
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

static BOOL same(NSString *left, NSString *right) {
  return left == right || [left isEqualToString:right];
}

static void notify(SWAccessibilityNode *node, SWAccessibilityState *old,
                   SWAccessibilityState *next) {
  if (!same(old.name, next.name))
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityTitleChangedNotification);
  if (!same(old.help, next.help) || !same(old.error, next.error) ||
      ![old.labels isEqualToArray:next.labels] ||
      ![old.controls isEqualToArray:next.controls] ||
      ![old.titleFor isEqualToArray:next.titleFor])
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityLayoutChangedNotification);
  if (!same(old.value, next.value) || old.data.checked != next.data.checked ||
      old.data.flags != next.data.flags)
    NSAccessibilityPostNotification(node,
                                    NSAccessibilityValueChangedNotification);
  if (old.data.geometry != next.data.geometry ||
      old.data.visible != next.data.visible)
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

int64_t scoop_window_accessibility_commit(void *pointer, int64_t focused) {
  @autoreleasepool {
    @try {
      SWAccessibilityTree *tree = (__bridge SWAccessibilityTree *)pointer;
      if (!tree.pending || tree.closed)
        return -4;
      NSDictionary *previous = tree.published;
      NSDictionary<NSNumber *, SWAccessibilityState *> *changes = tree.pending;
      NSSet<NSNumber *> *removed = tree.removed;
      NSMutableDictionary *next = [previous mutableCopy];
      for (NSNumber *number in removed)
        [next removeObjectForKey:number];
      [next addEntriesFromDictionary:changes];
      int64_t oldFocus = tree.focused;
      tree.published = next;
      tree.focused = focused;
      tree.pending = nil;
      tree.removed = nil;
      BOOL structure = removed.count != 0;
      for (NSNumber *number in changes) {
        SWAccessibilityState *old = previous[number];
        SWAccessibilityState *value = changes[number];
        structure |= !old || old.data.parent != value.data.parent ||
                     old.data.order != value.data.order ||
                     ((old.data.flags ^ value.data.flags) & SWAXHidden) != 0;
        if (old)
          notify([tree element:number.longLongValue], old, value);
      }
      for (NSNumber *number in removed) {
        SWAccessibilityNode *node = tree.elements[number];
        if (node)
          NSAccessibilityPostNotification(
              node, NSAccessibilityUIElementDestroyedNotification);
        [tree.elements removeObjectForKey:number];
        [tree.retiring removeObject:number];
      }
      if (structure)
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
  tree.view.semanticRoot = nil;
  tree.view = nil;
  tree.root.retired = YES;
  for (SWAccessibilityNode *node in tree.elements.allValues)
    node.retired = YES;
  [tree.elements removeAllObjects];
  [tree.retiring removeAllObjects];
  tree.published = @{};
  tree.pending = nil;
  tree.removed = nil;
}
