#import <AppKit/AppKit.h>
#include <stdint.h>

int32_t scoop_ax_test_image(void *pointer) {
  NSView *view = (__bridge NSView *)pointer;
  id root = view.accessibilityChildren.firstObject;
  id image = nil;
  for (id child in [root accessibilityChildren])
    if ([[child accessibilityLabel] isEqualToString:@"Compass"])
      image = child;
  if (![[image accessibilityRole] isEqualToString:NSAccessibilityImageRole])
    return 1;
  if (![[image accessibilityLabel] isEqualToString:@"Compass"] ||
      ![[image accessibilityHelp] isEqualToString:@"North marker"])
    return 2;
  if (!NSEqualRects(
          [image accessibilityFrame],
          NSAccessibilityFrameInView(view, NSMakeRect(10, 20, 100, 80))))
    return 3;
  if ([image isAccessibilitySelectorAllowed:@selector(
                                                accessibilityPerformPress)] ||
      [image
          isAccessibilitySelectorAllowed:@selector(setAccessibilityFocused:)])
    return 4;
  return 0;
}
