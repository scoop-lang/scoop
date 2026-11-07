#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_test_native_ui_menu(void *pointer, int32_t activate) {
  NSMenuItem *root = NSApp.mainMenu.itemArray.firstObject;
  NSMenuItem *item = root.submenu.itemArray.firstObject;
  if (!item)
    return 0;
  int32_t label = [item.title isEqualToString:@"App"]       ? 1
                  : [item.title isEqualToString:@"Window"]  ? 2
                  : [item.title isEqualToString:@"Changed"] ? 3
                                                            : -1;
  if (activate && item.action)
    [NSApp sendAction:item.action to:item.target from:item];
  return label;
}
