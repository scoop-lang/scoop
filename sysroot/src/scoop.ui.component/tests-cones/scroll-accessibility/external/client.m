#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#include <unistd.h>

#define CHECK(condition)                                                       \
  do {                                                                         \
    if (!(condition)) {                                                        \
      fprintf(stderr, "AX check failed at line %d: %s\n", __LINE__,            \
              #condition);                                                     \
      exit(1);                                                                 \
    }                                                                          \
  } while (0)

static AXUIElementRef ax(id element) {
  return (__bridge AXUIElementRef)element;
}

static id value(id element, CFStringRef attribute) {
  CFTypeRef result = NULL;
  if (element)
    AXUIElementCopyAttributeValue(ax(element), attribute, &result);
  return CFBridgingRelease(result);
}

static NSArray *slice(id element, CFStringRef attribute, CFIndex index,
                      CFIndex n) {
  CFArrayRef result = NULL;
  if (element)
    AXUIElementCopyAttributeValues(ax(element), attribute, index, n, &result);
  return CFBridgingRelease(result);
}

static CFIndex count(id element, CFStringRef attribute) {
  CFIndex size = -1;
  CHECK(AXUIElementGetAttributeValueCount(ax(element), attribute, &size) ==
        kAXErrorSuccess);
  return size;
}

static NSString *label(id element) {
  NSString *description = value(element, kAXDescriptionAttribute);
  return description.length ? description : value(element, kAXTitleAttribute);
}

static id find(id element, NSString *role, NSString *name) {
  NSString *actual = value(element, kAXRoleAttribute);
  if ([role isEqual:actual] && (!name || [name isEqual:label(element)]))
    return element;
  // Discover only the window controls. Never enumerate a collection to find its
  // navigator.
  if ([actual isEqual:@"AXList"])
    return nil;
  for (id child in slice(element, kAXChildrenAttribute, 0, 32)) {
    id found = find(child, role, name);
    if (found)
      return found;
  }
  return nil;
}

static void perform(id element, CFStringRef action) {
  CHECK(element &&
        AXUIElementPerformAction(ax(element), action) == kAXErrorSuccess);
}

static void performNamed(id element, NSString *title) {
  CFArrayRef actions = NULL;
  CHECK(AXUIElementCopyActionNames(ax(element), &actions) == kAXErrorSuccess);
  for (NSString *action in CFBridgingRelease(actions)) {
    CFStringRef description = NULL;
    AXUIElementCopyActionDescription(ax(element), (__bridge CFStringRef)action,
                                     &description);
    if ([title isEqual:CFBridgingRelease(description)]) {
      perform(element, (__bridge CFStringRef)action);
      return;
    }
  }
  fprintf(stderr, "Missing AX action %s\n", title.UTF8String);
  exit(1);
}

static void setRow(id navigator, NSNumber *row) {
  CHECK(AXUIElementSetAttributeValue(ax(navigator), kAXValueAttribute,
                                     (__bridge CFNumberRef)row) ==
        kAXErrorSuccess);
}

static BOOL hasFrame(id element) {
  id size = value(element, kAXSizeAttribute);
  CGSize extent = CGSizeZero;
  return size &&
         AXValueGetValue((__bridge AXValueRef)size, kAXValueCGSizeType,
                         &extent) &&
         extent.width > 0 && extent.height > 0;
}

static BOOL waitValue(id element, CFStringRef attribute, NSNumber *expected) {
  for (int attempt = 0; attempt < 100; ++attempt) {
    if ([expected isEqual:value(element, attribute)])
      return YES;
    usleep(50000);
  }
  return NO;
}

static NSArray *readPage(id list, NSUInteger first, NSUInteger size) {
  CFStringRef attributes[] = {kAXChildrenAttribute, kAXRowsAttribute,
                              CFSTR("AXChildrenInNavigationOrder")};
  NSArray *children = nil;
  CFAbsoluteTime start = CFAbsoluteTimeGetCurrent();
  for (NSUInteger i = 0; i < sizeof(attributes) / sizeof(attributes[0]); ++i) {
    CHECK(count(list, attributes[i]) == (CFIndex)size);
    NSArray *rows = value(list, attributes[i]);
    CHECK(rows.count == size);
    CHECK([@(first) isEqual:value(rows.firstObject, kAXIndexAttribute)]);
    CHECK([@(first + size - 1)
        isEqual:value(rows.lastObject, kAXIndexAttribute)]);
    CHECK([list isEqual:value(rows.lastObject, kAXParentAttribute)]);
    CHECK(
        [@[ rows.lastObject ] isEqual:slice(list, attributes[i], size - 1, 3)]);
    if (children)
      CHECK([children isEqual:rows]);
    else
      children = rows;
  }
  printf("AXChildren / AXRows / AXChildrenInNavigationOrder: %lu..%lu (%lu "
         "rows), %.1f ms\n",
         first, first + size - 1, size,
         (CFAbsoluteTimeGetCurrent() - start) * 1000);
  return children;
}

int main(int argc, const char **argv) {
  @autoreleasepool {
    CHECK(argc == 2);
    CHECK(AXIsProcessTrusted());
    id app = CFBridgingRelease(AXUIElementCreateApplication(atoi(argv[1])));
    CHECK(AXUIElementSetMessagingTimeout(ax(app), 3) == kAXErrorSuccess);
    id window = nil, list = nil;
    for (int attempt = 0; attempt < 150 && !list; ++attempt) {
      window = [value(app, kAXWindowsAttribute) firstObject];
      list = find(window, @"AXList", nil);
      if (!list)
        usleep(100000);
    }
    CHECK(list &&
          AXUIElementSetMessagingTimeout(ax(list), 3) == kAXErrorSuccess);
    id group = value(list, kAXParentAttribute);
    CHECK([@"AXGroup" isEqual:value(group, kAXRoleAttribute)]);
    id navigator = find(group, @"AXIncrementor", @"Go to row");
    CHECK(navigator && [@1 isEqual:value(navigator, kAXMinValueAttribute)]);
    CHECK([@100000000 isEqual:value(navigator, kAXMaxValueAttribute)]);
    CHECK([@"Rows 1–100 of 100000000" isEqual:label(group)]);
    id measure = find(window, @"AXButton", @"Read counters");
    // The first accepted frame must preserve a page chosen as the window opens.
    setRow(navigator, @100000000);
    for (int attempt = 0; attempt < 100 && !hasFrame(measure); ++attempt)
      usleep(50000);
    CHECK(hasFrame(measure) &&
          [@100000000 isEqual:value(navigator, kAXValueAttribute)]);
    setRow(navigator, @1);
    CHECK([value(list, kAXVisibleChildrenAttribute) count] > 0);
    perform(measure, kAXPressAction);
    id focused = value(app, kAXFocusedUIElementAttribute);
    id first = [readPage(list, 0, 100) firstObject];
    CHECK([@[ first ] isEqual:value(list, kAXSelectedChildrenAttribute)]);
    perform(navigator, kAXIncrementAction);
    CHECK([@101 isEqual:value(navigator, kAXValueAttribute)]);
    perform(navigator, kAXDecrementAction);
    CHECK([@1 isEqual:value(navigator, kAXValueAttribute)]);
    CHECK(
        [first isEqual:[slice(list, kAXChildrenAttribute, 0, 1) firstObject]]);

    setRow(navigator, @50000001);
    CHECK([@50000000
        isEqual:value([slice(list, kAXRowsAttribute, 0, 1) firstObject],
                      kAXIndexAttribute)]);
    performNamed(navigator, @"Last Page");
    CHECK([@100000000 isEqual:value(navigator, kAXValueAttribute)]);
    performNamed(navigator, @"First Page");
    CHECK([@1 isEqual:value(navigator, kAXValueAttribute)]);
    setRow(navigator, @100000000);
    CHECK([@100000000 isEqual:value(navigator, kAXValueAttribute)]);
    CHECK([@"Rows 99999901–100000000 of 100000000" isEqual:label(group)]);
    id last = [readPage(list, 99999900, 100) lastObject];
    CHECK([@"Row 99999999" isEqual:label(last)]);
    CHECK([@"row-99999999" isEqual:value(last, kAXIdentifierAttribute)]);
    CHECK([@[] isEqual:value(list, kAXVisibleChildrenAttribute)]);
    CHECK([@[ last ] isEqual:value(list, kAXSelectedChildrenAttribute)]);
    id after = value(app, kAXFocusedUIElementAttribute);
    CHECK(focused == after || [focused isEqual:after]);
    perform(measure, kAXPressAction);

    setRow(navigator, @1);
    perform(find(window, @"AXButton", @"Scroll to end"), kAXPressAction);
    BOOL visible = NO;
    for (int attempt = 0; attempt < 100 && !visible; ++attempt) {
      visible = [value(list, kAXVisibleChildrenAttribute) containsObject:last];
      if (!visible)
        usleep(50000);
    }
    CHECK(visible &&
          [last isEqual:[slice(list, kAXRowsAttribute, 99, 1) firstObject]]);
    perform(find(window, @"AXButton", @"4097 rows"), kAXPressAction);
    CHECK(waitValue(navigator, kAXMaxValueAttribute, @4097));
    CHECK([value(navigator, kAXValueAttribute) longLongValue] <= 4097);
    setRow(navigator, @1);
    NSArray *selected = value(list, kAXSelectedChildrenAttribute);
    CHECK(selected.count == 1 && [first isEqual:selected.firstObject]);
    setRow(navigator, @4097);
    readPage(list, 4000, 97);

    perform(find(window, @"AXButton", @"4096 rows"), kAXPressAction);
    for (int attempt = 0;
         attempt < 100 && count(list, kAXRowsAttribute) != 4096; ++attempt)
      usleep(50000);
    CHECK(count(list, kAXChildrenAttribute) == 4096);
    CHECK(
        [@4095 isEqual:value([slice(list, CFSTR("AXChildrenInNavigationOrder"),
                                    4095, 1) firstObject],
                             kAXIndexAttribute)]);
    CHECK(!value(navigator, kAXRoleAttribute));
    CHECK(![group isEqual:value(list, kAXParentAttribute)]);
  }
  return 0;
}
