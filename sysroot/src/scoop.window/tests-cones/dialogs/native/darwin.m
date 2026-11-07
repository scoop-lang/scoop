#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_test_file_dialog(NSWindow *owner, int32_t action, int32_t kind) {
  if (action == 4)
    return 0;
  if (action == 3)
    return owner.canBecomeKeyWindow;
  int count = 0;
  for (NSWindow *window in NSApp.windows) {
    if (![window isKindOfClass:NSSavePanel.class] || !window.visible ||
        ![window.title isEqualToString:@"Scoop file probe"])
      continue;
    NSSavePanel *panel = (NSSavePanel *)window;
    int actual = 2;
    if ([panel isKindOfClass:NSOpenPanel.class]) {
      NSOpenPanel *open = (NSOpenPanel *)panel;
      actual = open.canChooseDirectories && !open.canChooseFiles ? 3
               : open.allowsMultipleSelection                    ? 1
                                                                 : 0;
    }
    if ((kind >= 0 && kind != actual) || (owner && panel.sheetParent != owner))
      continue;
    ++count;
    if (action == 1) {
      [panel cancel:nil];
      return 1;
    }
    if (action == 2)
      return owner ? panel.sheetParent == owner : NSApp.modalWindow == panel;
  }
  return count;
}

int32_t scoop_test_file_choose(const uint8_t *path, int64_t length) {
  return 0;
}
