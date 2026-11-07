#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_test_tray_click(void *pointer) {
  id tray = (__bridge id)pointer;
  NSStatusItem *item = [tray valueForKey:@"item"];
  NSButton *button = item.button;
  return [button sendAction:button.action to:button.target];
}

int32_t scoop_test_tray_image(void *pointer) {
  id tray = (__bridge id)pointer;
  NSImage *image = [tray valueForKey:@"image"];
  return image.template && image.representations.count == 2 &&
         image.size.width == 16 && image.size.height == 16;
}

int32_t scoop_test_tray_select(void *pointer) {
  id tray = (__bridge id)pointer;
  NSMenu *menu = [tray valueForKey:@"menu"];
  NSMenuItem *item = menu.itemArray.firstObject;
  BOOL sent = item && [NSApp sendAction:item.action to:item.target from:item];
  [menu cancelTrackingWithoutAnimation];
  return sent;
}
