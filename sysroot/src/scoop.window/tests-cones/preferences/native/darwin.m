#import <AppKit/AppKit.h>
#import <objc/runtime.h>
#include <stdint.h>

static IMP original_motion;
static BOOL motion;

static BOOL reduced_motion(id object, SEL selector) { return motion; }

int32_t scoop_test_preferences_motion(void) {
  return NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion;
}

int32_t scoop_test_preferences_scrollbars(void) {
  return NSScroller.preferredScrollerStyle == NSScrollerStyleOverlay;
}

void scoop_test_preferences_change(int32_t value) {
  Method method = class_getInstanceMethod(
      NSWorkspace.class, @selector(accessibilityDisplayShouldReduceMotion));
  if (!original_motion)
    original_motion = method_setImplementation(method, (IMP)reduced_motion);
  motion = value != 0;
  // Change only this process's query result, then use the real notification
  // path.
  [NSWorkspace.sharedWorkspace.notificationCenter
      postNotificationName:
          NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification
                    object:nil];
}

void scoop_test_preferences_restore(void) {
  if (original_motion) {
    Method method = class_getInstanceMethod(
        NSWorkspace.class, @selector(accessibilityDisplayShouldReduceMotion));
    method_setImplementation(method, original_motion);
    original_motion = NULL;
  }
}
