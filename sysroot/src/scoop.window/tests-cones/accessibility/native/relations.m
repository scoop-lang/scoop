#import <AppKit/AppKit.h>
#include <stdint.h>

#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      return __LINE__;                                                         \
  } while (0)

@interface SWRelationProbe : NSObject
@property(nonatomic, strong) NSAccessibilityElement *root, *field, *label,
    *alias, *suffix, *button;
@end
@implementation SWRelationProbe
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

void *scoop_ax_relations_open(void *pointer) {
  NSView *view = (__bridge NSView *)pointer;
  SWRelationProbe *probe = [SWRelationProbe new];
  probe.root = view.accessibilityChildren.firstObject;
  probe.field = find(probe.root, @"relation-field");
  probe.label = find(probe.root, @"relation-label");
  probe.alias = find(probe.root, @"relation-alias");
  probe.suffix = find(probe.root, @"relation-suffix");
  probe.button = find(probe.root, @"relation-button");
  return (__bridge_retained void *)probe;
}

int32_t scoop_ax_relations_check(void *pointer, int32_t phase) {
  SWRelationProbe *probe = (__bridge SWRelationProbe *)pointer;
  CHECK(probe.field && probe.label && probe.alias && probe.suffix &&
        probe.button);
  switch (phase) {
  case 0:
  case 1:
    CHECK([probe.field.accessibilityLabel
        isEqualToString:phase ? @"姓名 name" : @"User name"]);
    CHECK([probe.field.accessibilityHelp
        isEqualToString:@"Use full name\nRequired"]);
    CHECK(!probe.field.accessibilityTitleUIElement);
    CHECK(([probe.field.accessibilityLinkedUIElements
        isEqualToArray:@[ probe.button, probe.suffix ]]));
    CHECK([probe.label.accessibilityServesAsTitleForUIElements
        isEqualToArray:@[ probe.alias ]]);
    CHECK(probe.alias.accessibilityTitleUIElement == probe.label);
    break;
  case 2:
  case 4:
    CHECK([probe.field.accessibilityLabel
        isEqualToString:phase == 2 ? @"姓名" : @"Ignored"]);
    CHECK(probe.field.accessibilityTitleUIElement == probe.label);
    CHECK(probe.label.accessibilityServesAsTitleForUIElements.count == 2);
    CHECK([probe.label.accessibilityServesAsTitleForUIElements
        containsObject:probe.field]);
    CHECK([probe.field.accessibilityLinkedUIElements
        isEqualToArray:@[ probe.button ]]);
    if (phase == 4)
      CHECK([probe.button.accessibilityLinkedUIElements
          isEqualToArray:@[ probe.field ]]);
    break;
  case 3:
    CHECK([probe.field.accessibilityLabel isEqualToString:@""]);
    CHECK([probe.field.accessibilityHelp isEqualToString:@""]);
    CHECK(!probe.field.accessibilityTitleUIElement);
    CHECK([probe.label.accessibilityServesAsTitleForUIElements
        isEqualToArray:@[ probe.alias ]]);
    break;
  case 5:
    CHECK([probe.field.accessibilityLabel isEqualToString:@"Hidden label"]);
    CHECK(!probe.field.accessibilityTitleUIElement);
    CHECK(!probe.label.isAccessibilityElement);
    CHECK(probe.label.accessibilityServesAsTitleForUIElements.count == 0);
    break;
  case 6:
    CHECK(!probe.field.accessibilityLabel);
    CHECK(!probe.field.accessibilityTitleUIElement);
    CHECK(!probe.label.accessibilityRole);
    CHECK(!probe.alias.accessibilityLabel);
    break;
  case 7:
    CHECK([probe.field.accessibilityLabel isEqualToString:@"Replacement"]);
    CHECK(probe.field.accessibilityTitleUIElement != probe.label);
    CHECK(probe.field.accessibilityTitleUIElement ==
          find(probe.root, @"relation-label"));
    CHECK(!probe.label.accessibilityRole);
    break;
  case 8:
    CHECK(!probe.field.accessibilityLabel);
    break;
  case 9:
    CHECK([probe.field.accessibilityLabel isEqualToString:@""]);
    break;
  case 10:
    CHECK(!probe.field.accessibilityRole);
    CHECK(!probe.field.accessibilityTitleUIElement);
    CHECK(probe.field.accessibilityLinkedUIElements.count == 0);
    CHECK(!probe.alias.accessibilityRole);
    break;
  case 11:
    CHECK([probe.field.accessibilityLabel isEqualToString:@"姓名"]);
    CHECK([probe.field.accessibilityHelp
        isEqualToString:@"Include your family name\nName is required"]);
    break;
  case 12:
    CHECK(probe.field.accessibilityLinkedUIElements.count == 0);
    CHECK(!probe.button.accessibilityRole);
    CHECK([probe.field.accessibilityLabel isEqualToString:@"Ignored"]);
    break;
  default:
    return -1;
  }
  return 0;
}

void scoop_ax_relations_close(void *pointer) {
  id object = (__bridge_transfer id)pointer;
  (void)object;
}
