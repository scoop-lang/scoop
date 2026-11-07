#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_test_modal_query(void *pointer, void *parent, int32_t field,
                               int32_t value) {
  NSWindow *window = (__bridge NSWindow *)pointer;
  switch (field) {
  case 0:
    return (__bridge void *)(window.sheetParent ?: window.parentWindow) ==
           parent;
  case 1:
    return !window.ignoresMouseEvents && window.canBecomeKeyWindow;
  case 2:
    return [window isKindOfClass:NSPanel.class];
  case 3:
    return window.sheet;
  case 4:
    [window performClose:nil];
    return 1;
  case 5:
    return NSApp.modalWindow == window;
  case 10:
    window.ignoresMouseEvents = !value;
    return 1;
  case 20:
    return 0;
  default:
    return -1;
  }
}
