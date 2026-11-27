#import "collection_rows.h"

@interface ScoopCollectionActionProbe : NSObject
@property(nonatomic, strong) id root, list, row, other, child, replacement;
@property(nonatomic, strong) NSAccessibilityCustomAction *realize, *reveal;
@property(nonatomic) BOOL closed;
@end
@implementation ScoopCollectionActionProbe
@end

static id findList(id node) {
  if ([[node accessibilityRole] isEqual:NSAccessibilityListRole])
    return node;
  for (id child in [node accessibilityChildren]) {
    id found = findList(child);
    if (found)
      return found;
  }
  return nil;
}
static BOOL perform(NSAccessibilityCustomAction *action) {
  id target = action.target;
  SEL selector = action.selector;
  if (!target || !selector)
    return NO;
  BOOL (*method)(id, SEL) =
      (BOOL (*)(id, SEL))[target methodForSelector:selector];
  return method && method(target, selector);
}
static BOOL visible(ScoopCollectionActionProbe *probe, id row) {
  return [[probe.list accessibilityVisibleRows] containsObject:row];
}

#define REQUIRE(condition)                                                     \
  do {                                                                         \
    if (!(condition))                                                          \
      return __LINE__;                                                         \
  } while (0)

static int32_t check(ScoopCollectionActionProbe *probe, int32_t phase,
                     int32_t (*callback)(void *, int32_t, int32_t),
                     void *context) {
  if (phase == 0)
    probe.list = findList(probe.root);
  REQUIRE(probe.list);
  id window = [probe.list accessibilityWindow];
  id responder = [window firstResponder];
  switch (phase) {
  case 0: {
    probe.row = at(probe.list, 50000);
    REQUIRE(probe.row);
    NSArray *actions = [probe.row accessibilityCustomActions];
    REQUIRE(actions.count == 2);
    probe.realize = actions[0];
    probe.reveal = actions[1];
    REQUIRE([probe.row
        isAccessibilitySelectorAllowed:@selector(setAccessibilityFocused:)]);
    REQUIRE([probe.row accessibilityChildren].count == 0);
    REQUIRE(perform(probe.realize));
    REQUIRE([probe.row accessibilityChildren].count == 1);
    probe.child = [probe.row accessibilityChildren][0];
    REQUIRE(
        [[probe.child accessibilityRole] isEqual:NSAccessibilityButtonRole]);
    REQUIRE([probe.child accessibilityParent] == probe.row);
    REQUIRE(![probe.row
        isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    break;
  }
  case 1:
    if ([probe.row accessibilityChildren].count != 0)
      return -1;
    REQUIRE(![probe.child isAccessibilityElement]);
    REQUIRE(![probe.child accessibilityPerformPress]);
    REQUIRE(!perform(probe.reveal));
    break;
  case 2:
    if (!visible(probe, probe.row))
      return -1;
    REQUIRE(perform(probe.reveal));
    REQUIRE([probe.row accessibilityChildren].count == 1);
    REQUIRE([[probe.row accessibilityChildren][0] accessibilityPerformPress]);
    REQUIRE(![probe.row isAccessibilityFocused]);
    break;
  case 3: {
    probe.other = at(probe.list, 75000);
    REQUIRE(probe.other);
    // Cancel a far-row focus request before any frame can grant focus.
    [probe.other setAccessibilityFocused:YES];
    REQUIRE(![probe.other isAccessibilityFocused]);
    callback(context, -1, 0);
    [probe.other setAccessibilityFocused:NO];
    REQUIRE(!perform([probe.other accessibilityCustomActions][1]));
    id bar = [[[probe.list accessibilityParent] accessibilityParent]
        accessibilityVerticalScrollBar];
    REQUIRE(bar);
    [bar setAccessibilityValue:@0.0];
    break;
  }
  case 4:
    if (visible(probe, probe.row) ||
        [probe.other accessibilityChildren].count != 0)
      return -1;
    REQUIRE([probe.row accessibilityChildren].count == 0);
    REQUIRE(![probe.other isAccessibilityFocused]);
    break;
  case 5:
    REQUIRE(![probe.row isAccessibilityEnabled]);
    REQUIRE(!perform(probe.realize));
    REQUIRE(!perform(probe.reveal));
    [probe.row setAccessibilityFocused:YES];
    REQUIRE(![probe.row isAccessibilityFocused]);
    REQUIRE([probe.row accessibilityChildren].count == 0);
    break;
  case 6:
    REQUIRE(at(probe.list, 50000) == probe.row);
    REQUIRE([probe.row accessibilityIndex] == 50000);
    REQUIRE(!perform(probe.reveal));
    break;
  case 7:
    if (!visible(probe, probe.row))
      return -1;
    REQUIRE(at(probe.list, 49999) == probe.row);
    REQUIRE([probe.row accessibilityIndex] == 49999);
    REQUIRE(perform(probe.reveal));
    REQUIRE([[probe.row accessibilityLabel] isEqual:@"Row 50000"]);
    REQUIRE([[probe.row accessibilityChildren][0] accessibilityLabel] != nil);
    break;
  case 8:
    REQUIRE(![probe.row isAccessibilityElement]);
    REQUIRE(!perform(probe.realize));
    REQUIRE(!perform(probe.reveal));
    break;
  case 9:
    probe.replacement = at(probe.list, 49999);
    REQUIRE(probe.replacement && probe.replacement != probe.row);
    REQUIRE(!perform(probe.realize));
    REQUIRE(perform([probe.replacement accessibilityCustomActions][0]));
    REQUIRE([probe.replacement accessibilityChildren].count == 1);
    probe.other = at(probe.list, 24999);
    REQUIRE(!perform([probe.other accessibilityCustomActions][1]));
    break;
  case 10:
    REQUIRE(![probe.replacement isAccessibilityElement]);
    REQUIRE(![probe.other isAccessibilityElement]);
    REQUIRE(!perform(probe.realize));
    REQUIRE(!perform(probe.reveal));
    break;
  default:
    return __LINE__;
  }
  REQUIRE([window firstResponder] == responder);
  return 0;
}

void *scoop_ui_collection_action_probe(void *root) {
  ScoopCollectionActionProbe *probe = [ScoopCollectionActionProbe new];
  probe.root = (__bridge id)root;
  return (__bridge_retained void *)probe;
}
void scoop_ui_collection_action_post(void *pointer, int32_t phase,
                                     int32_t (*callback)(void *, int32_t,
                                                         int32_t),
                                     void *context) {
  ScoopCollectionActionProbe *probe =
      (__bridge ScoopCollectionActionProbe *)pointer;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!probe.closed) {
      @autoreleasepool {
        callback(context, phase, check(probe, phase, callback, context));
      }
    }
  });
}
void scoop_ui_collection_action_close(void *pointer) {
  ScoopCollectionActionProbe *probe =
      (__bridge_transfer ScoopCollectionActionProbe *)pointer;
  probe.closed = YES;
}
