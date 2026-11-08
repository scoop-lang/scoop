#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_test_font_focus(void *pointer) {
  NSWindow *window = (__bridge NSWindow *)pointer;
  // A CLI test has no foreground app launching it on the user's behalf.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
  [NSRunningApplication.currentApplication
      activateWithOptions:NSApplicationActivateIgnoringOtherApps];
#pragma clang diagnostic pop
  [window makeKeyAndOrderFront:nil];
  return (NSApp.isActive ? 1 : 0) | (window.isKeyWindow ? 2 : 0);
}
