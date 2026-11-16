#include <ApplicationServices/ApplicationServices.h>
#include <stdio.h>
#include <stdlib.h>

static AXError last_error;
#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value)) {                                                            \
      fprintf(stderr, "AX client check failed at line %d (status %d)\n",       \
              __LINE__, last_error);                                           \
      return 1;                                                                \
    }                                                                          \
  } while (0)

static CFTypeRef copy(AXUIElementRef node, CFStringRef key) {
  CFTypeRef value = NULL;
  last_error = AXUIElementCopyAttributeValue(node, key, &value);
  return last_error == kAXErrorSuccess ? value : NULL;
}

static int string_is(AXUIElementRef node, CFStringRef key,
                     CFStringRef expected) {
  CFTypeRef value = copy(node, key);
  int result = value && CFGetTypeID(value) == CFStringGetTypeID() &&
               CFEqual(value, expected);
  if (value)
    CFRelease(value);
  return result;
}

static int number_is(AXUIElementRef node, CFStringRef key, int expected) {
  CFTypeRef value = copy(node, key);
  int number = -1;
  int result = value && CFGetTypeID(value) == CFNumberGetTypeID() &&
               CFNumberGetValue(value, kCFNumberIntType, &number) &&
               number == expected;
  if (value)
    CFRelease(value);
  return result;
}

static int boolean(AXUIElementRef node, CFStringRef key) {
  CFTypeRef value = copy(node, key);
  int result = value && CFGetTypeID(value) == CFBooleanGetTypeID()
                   ? CFBooleanGetValue(value)
                   : -1;
  if (value)
    CFRelease(value);
  return result;
}

static AXUIElementRef find(AXUIElementRef root, CFStringRef identifier) {
  if (string_is(root, CFSTR("AXIdentifier"), identifier)) {
    CFRetain(root);
    return root;
  }
  CFTypeRef children = copy(root, kAXChildrenAttribute);
  if (!children || CFGetTypeID(children) != CFArrayGetTypeID()) {
    if (children)
      CFRelease(children);
    return NULL;
  }
  AXUIElementRef result = NULL;
  for (CFIndex index = 0; index < CFArrayGetCount(children) && !result; ++index)
    result = find((AXUIElementRef)CFArrayGetValueAtIndex(children, index),
                  identifier);
  CFRelease(children);
  return result;
}

static int press(AXUIElementRef node) {
  last_error = AXUIElementPerformAction(node, kAXPressAction);
  return last_error == kAXErrorSuccess;
}

static int set(AXUIElementRef node, CFStringRef key, CFTypeRef value) {
  last_error = AXUIElementSetAttributeValue(node, key, value);
  return last_error == kAXErrorSuccess;
}

static int retired(AXUIElementRef node) {
  CFTypeRef role = copy(node, kAXRoleAttribute);
  if (role)
    CFRelease(role);
  return !role && last_error != kAXErrorSuccess;
}

static int run(AXUIElementRef app, AXUIElementRef window, int operation) {
  CFStringRef identifier = operation == 1 || operation == 7   ? CFSTR("check")
                           : operation >= 2 && operation <= 5 ? CFSTR("field")
                                                              : CFSTR("save");
  AXUIElementRef node = find(window, identifier);
  if (operation == 6) {
    CHECK(!node);
    return 0;
  }
  CHECK(node);
  switch (operation) {
  case 0:
    CHECK(string_is(node, kAXRoleAttribute, kAXButtonRole));
    CHECK(press(node));
    CHECK(string_is(node, kAXTitleAttribute, CFSTR("Saved")));
    break;
  case 1:
    CHECK(press(node));
    CHECK(number_is(node, kAXValueAttribute, 1));
    break;
  case 2:
    CHECK(set(node, kAXValueAttribute, CFSTR("deny")));
    CHECK(string_is(node, kAXValueAttribute, CFSTR("alpha")));
    break;
  case 3:
    CHECK(set(node, kAXValueAttribute, CFSTR("中文😀")));
    CHECK(string_is(node, kAXValueAttribute, CFSTR("中文😀")));
    break;
  case 4: {
    CHECK(set(node, kAXFocusedAttribute, kCFBooleanTrue));
    int node_focused = boolean(node, kAXFocusedAttribute);
    CFTypeRef focused_window = copy(app, kAXFocusedWindowAttribute);
    CFTypeRef focused_element = copy(app, kAXFocusedUIElementAttribute);
    int matches_window = focused_window && CFEqual(focused_window, window);
    int matches_element = focused_element && CFEqual(focused_element, node);
    if (focused_window)
      CFRelease(focused_window);
    if (focused_element)
      CFRelease(focused_element);
    if (node_focused != 1 || !matches_window || !matches_element)
      fprintf(stderr, "AX focus mismatch: node=%d window=%d target=%d\n",
              node_focused, matches_window, matches_element);
    CHECK(node_focused == 1 && matches_window && matches_element);
    break;
  }
  case 5:
    CHECK(set(node, kAXFocusedAttribute, kCFBooleanFalse));
    CHECK(!boolean(node, kAXFocusedAttribute));
    break;
  case 7:
    CHECK(!boolean(node, kAXEnabledAttribute));
    // AppKit may acknowledge the IPC call even when the BOOL action rejects it.
    press(node);
    CHECK(last_error == kAXErrorSuccess ||
          last_error == kAXErrorActionUnsupported);
    CHECK(number_is(node, kAXValueAttribute, 1));
    break;
  case 8:
  case 9:
  case 10:
    CHECK(press(node));
    CHECK(retired(node));
    press(node);
    CHECK(last_error == kAXErrorSuccess ||
          last_error == kAXErrorActionUnsupported ||
          last_error == kAXErrorInvalidUIElement);
    CHECK(retired(node));
    break;
  default:
    CFRelease(node);
    return 1;
  }
  CFRelease(node);
  return 0;
}

int main(int argc, char **argv) {
  CHECK(argc == 3);
  CHECK(AXIsProcessTrusted());
  AXUIElementRef app =
      AXUIElementCreateApplication((pid_t)strtol(argv[1], NULL, 10));
  CHECK(app);
  AXUIElementSetMessagingTimeout(app, 3.0f);
  CFTypeRef windows = copy(app, kAXWindowsAttribute);
  CHECK(windows && CFGetTypeID(windows) == CFArrayGetTypeID());
  CHECK(CFArrayGetCount(windows) == 1);
  AXUIElementRef window = (AXUIElementRef)CFArrayGetValueAtIndex(windows, 0);
  if (!string_is(window, kAXRoleAttribute, kAXWindowRole)) {
    CFTypeRef role = copy(window, kAXRoleAttribute);
    if (role) {
      CFShow(role);
      CFRelease(role);
    }
  }
  CHECK(string_is(window, kAXRoleAttribute, kAXWindowRole));
  int result = run(app, window, (int)strtol(argv[2], NULL, 10));
  CFRelease(windows);
  CFRelease(app);
  return result;
}
