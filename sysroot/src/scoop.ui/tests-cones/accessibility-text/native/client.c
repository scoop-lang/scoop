#include <ApplicationServices/ApplicationServices.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static AXError last_error;
#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value)) {                                                            \
      fprintf(stderr, "AX text check failed at line %d (status %d)\n",         \
              __LINE__, last_error);                                           \
      return 1;                                                                \
    }                                                                          \
  } while (0)

static CFTypeRef copy(AXUIElementRef node, CFStringRef key) {
  CFTypeRef value = NULL;
  last_error = AXUIElementCopyAttributeValue(node, key, &value);
  return value;
}

static CFTypeRef parameter(AXUIElementRef node, CFStringRef key,
                           CFTypeRef argument) {
  CFTypeRef value = NULL;
  last_error =
      AXUIElementCopyParameterizedAttributeValue(node, key, argument, &value);
  return value;
}

static int string_is(AXUIElementRef node, CFStringRef key,
                     CFStringRef expected) {
  CFTypeRef value = copy(node, key);
  int matches = value && CFGetTypeID(value) == CFStringGetTypeID() &&
                CFEqual(value, expected);
  if (value)
    CFRelease(value);
  return matches;
}

static int number_is(CFTypeRef value, int expected) {
  int actual = -1;
  int matches = value && CFGetTypeID(value) == CFNumberGetTypeID() &&
                CFNumberGetValue(value, kCFNumberIntType, &actual) &&
                actual == expected;
  if (value)
    CFRelease(value);
  return matches;
}

static int range_is(CFTypeRef value, CFIndex start, CFIndex length) {
  CFRange actual;
  int matches = value && CFGetTypeID(value) == AXValueGetTypeID() &&
                AXValueGetValue(value, kAXValueCFRangeType, &actual) &&
                actual.location == start && actual.length == length;
  if (value)
    CFRelease(value);
  return matches;
}

static CFTypeRef for_index(AXUIElementRef node, CFStringRef key, int index) {
  CFNumberRef argument = CFNumberCreate(NULL, kCFNumberIntType, &index);
  CFTypeRef value = parameter(node, key, argument);
  CFRelease(argument);
  return value;
}

static CFTypeRef for_range(AXUIElementRef node, CFStringRef key,
                           CFRange range) {
  AXValueRef argument = AXValueCreate(kAXValueCFRangeType, &range);
  CFTypeRef value = parameter(node, key, argument);
  CFRelease(argument);
  return value;
}

static int no_range(CFTypeRef value) {
  CFRange range;
  int unavailable = !value && (last_error == kAXErrorNoValue ||
                               last_error == kAXErrorIllegalArgument);
  if (value) {
    unavailable = CFGetTypeID(value) == AXValueGetTypeID() &&
                  AXValueGetValue(value, kAXValueCFRangeType, &range) &&
                  (range.location == kCFNotFound || range.location == LONG_MAX);
    CFRelease(value);
  }
  return unavailable;
}

static AXUIElementRef find(AXUIElementRef node, CFStringRef identifier) {
  if (string_is(node, CFSTR("AXIdentifier"), identifier)) {
    CFRetain(node);
    return node;
  }
  CFTypeRef children = copy(node, kAXChildrenAttribute);
  AXUIElementRef result = NULL;
  if (children && CFGetTypeID(children) == CFArrayGetTypeID()) {
    for (CFIndex index = 0; index < CFArrayGetCount(children) && !result;
         ++index)
      result = find(CFArrayGetValueAtIndex(children, index), identifier);
  }
  if (children)
    CFRelease(children);
  return result;
}

typedef struct {
  int selection, value;
} Notifications;

static void notified(AXObserverRef observer, AXUIElementRef node,
                     CFStringRef notification, void *context) {
  (void)observer;
  (void)node;
  Notifications *counts = context;
  if (CFEqual(notification, kAXSelectedTextChangedNotification))
    counts->selection += 1;
  if (CFEqual(notification, kAXValueChangedNotification))
    counts->value += 1;
}

static int change_direction(AXUIElementRef node, pid_t pid, int editor) {
  CFRange range = CFRangeMake(1, editor ? 7 : 3);
  CHECK(range_is(copy(node, kAXSelectedTextRangeAttribute), range.location,
                 range.length));
  CHECK(number_is(copy(node, kAXInsertionPointLineNumberAttribute), 0));
  AXObserverRef observer = NULL;
  last_error = AXObserverCreate(pid, notified, &observer);
  CHECK(last_error == kAXErrorSuccess);
  Notifications counts = {0};
  last_error = AXObserverAddNotification(
      observer, node, kAXSelectedTextChangedNotification, &counts);
  CHECK(last_error == kAXErrorSuccess);
  last_error = AXObserverAddNotification(observer, node,
                                         kAXValueChangedNotification, &counts);
  CHECK(last_error == kAXErrorSuccess);
  CFRunLoopAddSource(CFRunLoopGetCurrent(),
                     AXObserverGetRunLoopSource(observer),
                     kCFRunLoopDefaultMode);
  AXValueRef value = AXValueCreate(kAXValueCFRangeType, &range);
  last_error =
      AXUIElementSetAttributeValue(node, kAXSelectedTextRangeAttribute, value);
  CHECK(last_error == kAXErrorSuccess);
  CHECK(range_is(copy(node, kAXSelectedTextRangeAttribute), range.location,
                 range.length));
  CHECK(number_is(copy(node, kAXInsertionPointLineNumberAttribute),
                  editor ? 1 : 0));
  CFAbsoluteTime deadline = CFAbsoluteTimeGetCurrent() + 3.0;
  while (!counts.selection && CFAbsoluteTimeGetCurrent() < deadline)
    CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, true);
  CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.15, false);
  fprintf(stdout, "AX selection direction: selection=%d value=%d\n",
          counts.selection, counts.value);
  CHECK(counts.selection == 1 && counts.value == 0);
  // Repeating the same forward selection must not publish another change.
  last_error =
      AXUIElementSetAttributeValue(node, kAXSelectedTextRangeAttribute, value);
  CFRelease(value);
  CHECK(last_error == kAXErrorSuccess);
  CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.15, false);
  CHECK(counts.selection == 1 && counts.value == 0);
  CFRunLoopRemoveSource(CFRunLoopGetCurrent(),
                        AXObserverGetRunLoopSource(observer),
                        kCFRunLoopDefaultMode);
  CFRelease(observer);
  return 0;
}

static int geometry(AXUIElementRef node, int editor, CGRect expected) {
  CHECK(string_is(node, kAXRoleAttribute,
                  editor ? kAXTextFieldRole : kAXStaticTextRole));
  CHECK(string_is(node, kAXValueAttribute,
                  editor ? CFSTR("A😀é中\nZ") : CFSTR("A😀图éZ")));
  CHECK(number_is(copy(node, kAXNumberOfCharactersAttribute), editor ? 8 : 7));
  CHECK(range_is(copy(node, kAXVisibleCharacterRangeAttribute), 0,
                 editor ? 8 : 7));
  CFArrayRef attributes = NULL;
  last_error = AXUIElementCopyParameterizedAttributeNames(node, &attributes);
  CHECK(last_error == kAXErrorSuccess && attributes);
  CFStringRef required[] = {kAXRangeForIndexParameterizedAttribute,
                            kAXRangeForPositionParameterizedAttribute,
                            kAXBoundsForRangeParameterizedAttribute,
                            kAXLineForIndexParameterizedAttribute,
                            kAXRangeForLineParameterizedAttribute};
  for (size_t index = 0; index < sizeof(required) / sizeof(required[0]);
       ++index)
    CHECK(CFArrayContainsValue(attributes,
                               CFRangeMake(0, CFArrayGetCount(attributes)),
                               required[index]));
  CFRelease(attributes);
  CHECK(range_is(for_index(node, kAXRangeForIndexParameterizedAttribute, 1), 1,
                 2));
  CHECK(no_range(for_index(node, kAXRangeForIndexParameterizedAttribute, 2)));
  CHECK(range_is(
      for_index(node, kAXRangeForIndexParameterizedAttribute, editor ? 4 : 5),
      editor ? 3 : 4, 2));
  CHECK(range_is(for_index(node, kAXRangeForLineParameterizedAttribute, 0), 0,
                 7));
  if (editor) {
    CHECK(range_is(for_index(node, kAXRangeForLineParameterizedAttribute, 1), 7,
                   1));
    CHECK(number_is(for_index(node, kAXLineForIndexParameterizedAttribute, 7),
                    1));
  }
  CHECK(no_range(
      for_index(node, kAXRangeForLineParameterizedAttribute, editor ? 2 : 1)));
  CFTypeRef text = for_range(node, kAXStringForRangeParameterizedAttribute,
                             CFRangeMake(1, 2));
  CHECK(text && CFGetTypeID(text) == CFStringGetTypeID() &&
        CFEqual(text, CFSTR("😀")));
  CFRelease(text);
  text = for_range(node, kAXAttributedStringForRangeParameterizedAttribute,
                   CFRangeMake(1, 2));
  CHECK(text && CFGetTypeID(text) == CFAttributedStringGetTypeID() &&
        CFEqual(CFAttributedStringGetString(text), CFSTR("😀")));
  CFRelease(text);
  CFTypeRef value = for_range(node, kAXBoundsForRangeParameterizedAttribute,
                              CFRangeMake(editor ? 1 : 3, editor ? 2 : 1));
  CGRect actual;
  CHECK(value && CFGetTypeID(value) == AXValueGetTypeID() &&
        AXValueGetValue(value, kAXValueCGRectType, &actual));
  CFRelease(value);
  if (fabs(actual.origin.x - expected.origin.x) >= 0.001 ||
      fabs(actual.origin.y - expected.origin.y) >= 0.001 ||
      fabs(actual.size.width - expected.size.width) >= 0.001 ||
      fabs(actual.size.height - expected.size.height) >= 0.001) {
    fprintf(stderr,
            "AX bounds actual=(%.4f %.4f %.4f %.4f) expected=(%.4f %.4f %.4f "
            "%.4f)\n",
            actual.origin.x, actual.origin.y, actual.size.width,
            actual.size.height, expected.origin.x, expected.origin.y,
            expected.size.width, expected.size.height);
    return 1;
  }
  CGPoint point =
      CGPointMake(expected.origin.x + fmin(1.0, expected.size.width / 2),
                  expected.origin.y + expected.size.height / 2);
  AXValueRef argument = AXValueCreate(kAXValueCGPointType, &point);
  value = parameter(node, kAXRangeForPositionParameterizedAttribute, argument);
  CFRelease(argument);
  CHECK(range_is(value, 1, editor ? 2 : 3));
  fprintf(stdout, "AX text geometry and UTF-16 queries passed.\n");
  return 0;
}

int main(int argc, char **argv) {
  CHECK(argc == 7);
  CHECK(AXIsProcessTrusted());
  int editor = strcmp(argv[2], "document") == 0;
  CHECK(editor || strcmp(argv[2], "selectable") == 0);
  pid_t pid = (pid_t)strtol(argv[1], NULL, 10);
  AXUIElementRef app = AXUIElementCreateApplication(pid);
  AXUIElementSetMessagingTimeout(app, 5.0);
  CFTypeRef windows = copy(app, kAXWindowsAttribute);
  CHECK(windows && CFGetTypeID(windows) == CFArrayGetTypeID() &&
        CFArrayGetCount(windows) == 1);
  AXUIElementRef window = CFArrayGetValueAtIndex(windows, 0);
  CFStringRef identifier =
      CFStringCreateWithCString(NULL, argv[2], kCFStringEncodingUTF8);
  AXUIElementRef node = find(window, identifier);
  CFRelease(identifier);
  CHECK(node);
  CFTypeRef focus = copy(app, kAXFocusedUIElementAttribute);
  CGRect expected = CGRectMake(strtod(argv[3], NULL), strtod(argv[4], NULL),
                               strtod(argv[5], NULL), strtod(argv[6], NULL));
  CHECK(geometry(node, editor, expected) == 0);
  CHECK(change_direction(node, pid, editor) == 0);
  CFTypeRef focused = copy(node, kAXFocusedAttribute);
  CHECK(focused && CFEqual(focused, kCFBooleanFalse));
  CFRelease(focused);
  CFTypeRef current_focus = copy(app, kAXFocusedUIElementAttribute);
  CHECK((!focus && !current_focus) ||
        (focus && current_focus && CFEqual(focus, current_focus)));
  if (focus)
    CFRelease(focus);
  if (current_focus)
    CFRelease(current_focus);
  CFRelease(node);
  CFRelease(windows);
  CFRelease(app);
  return 0;
}
