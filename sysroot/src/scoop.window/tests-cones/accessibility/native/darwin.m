#import <AppKit/AppKit.h>
#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>

#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      return __LINE__;                                                         \
  } while (0)

@interface SWAccessibilityProbe : NSObject
@property(nonatomic, weak) NSView *view;
@property(nonatomic, strong) NSAccessibilityElement *root, *group, *text,
    *button, *box, *field, *secret, *offscreen;
@property(nonatomic) BOOL closed;
@property(nonatomic, strong) NSTask *task;
@property(nonatomic) int32_t lastOperation, lastResult;
@end
@implementation SWAccessibilityProbe
@end

static NSAccessibilityElement *find(NSAccessibilityElement *root,
                                    NSString *identifier) {
  if ([root.accessibilityIdentifier isEqualToString:identifier])
    return root;
  for (NSAccessibilityElement *child in root.accessibilityChildren) {
    NSAccessibilityElement *found = find(child, identifier);
    if (found)
      return found;
  }
  return nil;
}

void *scoop_ax_test_open(void *view) {
  SWAccessibilityProbe *probe = [SWAccessibilityProbe new];
  probe.view = (__bridge NSView *)view;
  probe.root = probe.view.accessibilityChildren.firstObject;
  probe.group = find(probe.root, @"group");
  probe.text = find(probe.root, @"text");
  probe.button = find(probe.root, @"save");
  probe.box = find(probe.root, @"check");
  probe.field = find(probe.root, @"field");
  probe.secret = find(probe.root, @"secret");
  probe.offscreen = find(probe.root, @"offscreen");
  probe.lastOperation = -1;
  return (__bridge_retained void *)probe;
}

int32_t scoop_ax_test_check(void *pointer, int32_t phase) {
  SWAccessibilityProbe *probe = (__bridge SWAccessibilityProbe *)pointer;
  CHECK(probe.root);
  if (phase == 1) {
    CHECK([probe.button.accessibilityLabel isEqualToString:@"Save"]);
    CHECK([probe.button
        isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    return 0;
  }
  if (phase == 2) {
    CHECK([probe.button.accessibilityLabel isEqualToString:@"Later"]);
    CHECK(![probe.button
        isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    return 0;
  }
  CHECK(probe.group && probe.text && probe.button && probe.box && probe.field &&
        probe.secret && probe.offscreen);
  CHECK(probe.root.accessibilityChildren.count == 5);
  CHECK(probe.group.accessibilityChildren.count == 2);
  CHECK(probe.group.accessibilityVisibleChildren.count == 1);
  CHECK(probe.root.accessibilityParent ==
        NSAccessibilityUnignoredAncestor(probe.view));
  CHECK(probe.button.accessibilityParent == probe.group);
  CHECK([probe.button.accessibilityRole
      isEqualToString:NSAccessibilityButtonRole]);
  CHECK([probe.button.accessibilityLabel isEqualToString:@"Save"]);
  CHECK([probe.button.accessibilityHelp isEqualToString:@"Write the document"]);
  CHECK([probe.text.accessibilityRole
      isEqualToString:NSAccessibilityStaticTextRole]);
  CHECK(probe.text.accessibilityLabel == nil);
  CHECK([probe.text.accessibilityValue isEqualToString:@""]);
  CHECK([probe.field.accessibilityValue isEqualToString:@"alpha"]);
  CHECK([probe.box.accessibilityValue isEqual:@0]);
  CHECK(probe.secret.isAccessibilityProtectedContent);
  CHECK(probe.secret.accessibilityValue == nil);
  CHECK(probe.offscreen.isAccessibilityElement);
  CHECK(probe.root.accessibilityVisibleChildren.count == 1);
  CHECK(!find(probe.root, @"hidden-child"));
  NSRect expected =
      NSAccessibilityFrameInView(probe.view, NSMakeRect(10, 20, 40, 30));
  CHECK(NSEqualRects(probe.button.accessibilityFrame, expected));
  CHECK(![probe.button
      isAccessibilitySelectorAllowed:@selector(accessibilityActivationPoint)]);
  CHECK(![probe.text
      isAccessibilitySelectorAllowed:@selector(accessibilityPerformPress)]);
  return 0;
}

int32_t scoop_ax_test_press(void *pointer) {
  return [((__bridge SWAccessibilityProbe *)
               pointer).button accessibilityPerformPress];
}

static int32_t operate(SWAccessibilityProbe *probe, int32_t operation) {
  switch (operation) {
  case 0:
    CHECK([probe.button accessibilityPerformPress]);
    CHECK([probe.button.accessibilityLabel isEqualToString:@"Saved"]);
    break;
  case 1:
    CHECK([probe.box accessibilityPerformPress]);
    CHECK([probe.box.accessibilityValue isEqual:@1]);
    break;
  case 2:
    [probe.field setAccessibilityValue:@"deny"];
    CHECK([probe.field.accessibilityValue isEqualToString:@"alpha"]);
    break;
  case 3:
    [probe.field setAccessibilityValue:@"中文😀"];
    CHECK([probe.field.accessibilityValue isEqualToString:@"中文😀"]);
    break;
  case 4:
    [probe.field setAccessibilityFocused:YES];
    CHECK(probe.field.isAccessibilityFocused == probe.view.window.keyWindow);
    break;
  case 5:
    [probe.field setAccessibilityFocused:NO];
    CHECK(!probe.field.isAccessibilityFocused);
    break;
  case 6:
    CHECK(!probe.button.isAccessibilityElement);
    CHECK(![probe.button accessibilityPerformPress]);
    CHECK(!find(probe.root, @"save"));
    break;
  case 7:
    CHECK(!probe.box.isAccessibilityEnabled);
    CHECK(![probe.box accessibilityPerformPress]);
    CHECK([probe.box.accessibilityValue isEqual:@1]);
    break;
  case 8:
    CHECK([probe.button accessibilityPerformPress]);
    CHECK(!probe.button.isAccessibilityElement);
    CHECK(!probe.button.accessibilityRole);
    CHECK(![probe.button accessibilityPerformPress]);
    CHECK(probe.group.accessibilityChildren.count == 1);
    break;
  case 9:
  case 10:
    CHECK([probe.button accessibilityPerformPress]);
    CHECK(!probe.root.isAccessibilityElement);
    CHECK(!probe.button.accessibilityRole);
    CHECK(![probe.button accessibilityPerformPress]);
    break;
  case 11:
    CHECK(![probe.button accessibilityPerformPress]);
    CHECK(!probe.button.accessibilityRole);
    break;
  default:
    return -1;
  }
  return 0;
}

void scoop_ax_test_post(void *pointer, int32_t operation,
                        int32_t (*callback)(void *, int32_t, int32_t),
                        void *context) {
  SWAccessibilityProbe *probe = (__bridge SWAccessibilityProbe *)pointer;
  const char *client = getenv("SCOOP_ACCESSIBILITY_CLIENT");
  if (client && *client && operation < 11) {
    NSTask *task = [NSTask new];
    probe.task = task;
    task.executableURL =
        [NSURL fileURLWithPath:[NSString stringWithUTF8String:client]];
    task.arguments = @[
      [NSString stringWithFormat:@"%d", getpid()],
      [NSString stringWithFormat:@"%d", operation]
    ];
    task.terminationHandler = ^(NSTask *finished) {
      int32_t result =
          finished.terminationStatus ? -100 - finished.terminationStatus : 0;
      dispatch_async(dispatch_get_main_queue(), ^{
        if (!probe.closed) {
          probe.task = nil;
          probe.lastOperation = operation;
          probe.lastResult = result;
          callback(context, operation, result);
        }
      });
    };
    NSError *error = nil;
    if (![task launchAndReturnError:&error]) {
      probe.lastOperation = operation;
      probe.lastResult = -3;
      callback(context, operation, -3);
    }
    return;
  }
  dispatch_async(dispatch_get_main_queue(), ^{
    @autoreleasepool {
      if (probe.closed)
        return;
      int32_t result;
      @try {
        result = operate(probe, operation);
      } @catch (NSException *exception) {
        result = -2;
      }
      probe.lastOperation = operation;
      probe.lastResult = result;
      if (!probe.closed)
        callback(context, operation, result);
    }
  });
}

int32_t scoop_ax_test_rebind(void *pointer, void *view) {
  SWAccessibilityProbe *probe = (__bridge SWAccessibilityProbe *)pointer;
  NSView *next = (__bridge NSView *)view;
  NSAccessibilityElement *root = next.accessibilityChildren.firstObject;
  CHECK(root && root != probe.root);
  CHECK(!probe.button.isAccessibilityElement);
  CHECK(!probe.button.accessibilityRole);
  probe.view = next;
  probe.root = root;
  probe.button = find(root, @"save");
  CHECK(probe.button);
  return 0;
}

void scoop_ax_test_refresh_button(void *pointer) {
  SWAccessibilityProbe *probe = (__bridge SWAccessibilityProbe *)pointer;
  probe.button = find(probe.root, @"save");
}

int32_t scoop_ax_test_last(void *pointer, int32_t operation) {
  SWAccessibilityProbe *probe = (__bridge SWAccessibilityProbe *)pointer;
  CHECK(probe.lastOperation == operation);
  return probe.lastResult;
}

void scoop_ax_test_close(void *pointer) {
  SWAccessibilityProbe *probe =
      (__bridge_transfer SWAccessibilityProbe *)pointer;
  probe.closed = YES;
  if (probe.task.running)
    [probe.task terminate];
  probe.task = nil;
}
