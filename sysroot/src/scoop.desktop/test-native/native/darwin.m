#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_desktop_test_application(int32_t field) {
  if (field == 0) {
    NSString *identifier = NSBundle.mainBundle.bundleIdentifier;
    return identifier ? [identifier isEqualToString:@"dev.scoop.desktop.tests"]
                      : -1;
  }
  return [NSApp.dockTile.badgeLabel isEqualToString:@"7"];
}
int32_t scoop_desktop_test_window(void *pointer, int32_t field) {
  NSWindow *window = (__bridge NSWindow *)pointer;
  if (field == 0)
    return window.miniwindowImage != nil;
  return [window.miniwindowTitle isEqualToString:@"Desktop caption"];
}
int32_t scoop_desktop_test_dock(void) {
  NSMenu *menu = [NSApp.delegate applicationDockMenu:NSApp];
  if (menu.numberOfItems != 1)
    return 0;
  NSMenuItem *item = [menu itemAtIndex:0];
  return [NSApp sendAction:item.action to:item.target from:item];
}

int32_t scoop_desktop_test_dock_popup(void *pointer) {
  NSMenu *menu = [NSApp.delegate applicationDockMenu:NSApp];
  if (!menu)
    return 0;
  NSWindow *window = (__bridge NSWindow *)pointer;
  [menu popUpMenuPositioningItem:nil
                      atLocation:NSMakePoint(20, 20)
                          inView:window.contentView];
  return 1;
}
