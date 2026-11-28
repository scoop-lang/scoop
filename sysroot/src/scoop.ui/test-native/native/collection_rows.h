#import <AppKit/AppKit.h>

// Exercise the public navigator before addressing the page-local row array.
static inline id rowNavigator(id list) {
  id parent = [list accessibilityParent];
  if ([[parent accessibilityRole] isEqual:NSAccessibilityGroupRole])
    for (id child in [parent accessibilityChildren])
      if ([[child accessibilityRole] isEqual:NSAccessibilityIncrementorRole])
        return child;
  return nil;
}

static inline id at(id list, NSUInteger index) {
  id navigator = rowNavigator(list);
  if (navigator) {
    [navigator setAccessibilityValue:@(index + 1)];
    NSArray *first =
        [list accessibilityArrayAttributeValues:NSAccessibilityRowsAttribute
                                          index:0
                                       maxCount:1];
    if (first.count != 1)
      return nil;
    index -= (NSUInteger)[first[0] accessibilityIndex];
  }
  NSArray *rows =
      [list accessibilityArrayAttributeValues:NSAccessibilityRowsAttribute
                                        index:index
                                     maxCount:1];
  return rows.count == 1 ? rows[0] : nil;
}
