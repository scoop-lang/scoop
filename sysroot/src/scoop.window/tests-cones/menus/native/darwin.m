#import <AppKit/AppKit.h>
#include <stdint.h>

static id tracking_observer;
static NSMenu *tracking_menu;

static NSMenuItem *find_item(NSMenu *menu, NSString *title) {
  for (NSMenuItem *item in menu.itemArray) {
    if ([item.title isEqualToString:title])
      return item;
    NSMenuItem *nested = find_item(item.submenu, title);
    if (nested)
      return nested;
  }
  return nil;
}

int32_t scoop_test_menu_action(const uint8_t *text, int64_t length) {
  NSString *title = [[NSString alloc] initWithBytes:text
                                             length:(NSUInteger)length
                                           encoding:NSUTF8StringEncoding];
  NSMenuItem *item = find_item(NSApp.mainMenu, title);
  return item && [NSApp sendAction:item.action to:item.target from:item];
}

int32_t scoop_test_closing_menu_role(void *pointer) {
  NSMenuItem *item = find_item((__bridge NSMenu *)pointer, @"Quit request");
  return item && [NSApp sendAction:item.action to:item.target from:item];
}

int32_t scoop_test_menu_shortcut(void *pointer) {
  NSWindow *window = (__bridge NSWindow *)pointer;
  NSEvent *event =
      [NSEvent keyEventWithType:NSEventTypeKeyDown
                             location:NSZeroPoint
                        modifierFlags:NSEventModifierFlagCommand
                            timestamp:NSProcessInfo.processInfo.systemUptime
                         windowNumber:window.windowNumber
                              context:nil
                           characters:@"o"
          charactersIgnoringModifiers:@"o"
                            isARepeat:NO
                              keyCode:31];
  return [window.contentView performKeyEquivalent:event];
}

int32_t scoop_test_native_menu_editor(void *pointer) {
  NSWindow *window = (__bridge NSWindow *)pointer;
  NSView *content = window.contentView;
  NSResponder *previous = window.firstResponder;
  NSTextField *field =
      [[NSTextField alloc] initWithFrame:NSMakeRect(8, 8, 180, 24)];
  field.stringValue = @"native edit target";
  [content addSubview:field];
  [window makeKeyWindow];
  [field selectText:nil];
  NSTextView *editor = (NSTextView *)window.firstResponder;
  editor.selectedRange = NSMakeRange(0, 0);
  NSMenuItem *item = find_item(NSApp.mainMenu, @"Native Select All");
  [item.menu update];
  BOOL enabled = item.enabled;
  BOOL sent = [NSApp sendAction:item.action to:item.target from:item];
  BOOL selected = editor.selectedRange.length == field.stringValue.length;
  [window makeFirstResponder:previous];
  [field removeFromSuperview];
  return enabled && sent && selected;
}

int64_t scoop_test_menu_property(void *pointer, const uint8_t *text,
                                 int64_t length, int32_t field) {
  NSString *title = [[NSString alloc] initWithBytes:text
                                             length:(NSUInteger)length
                                           encoding:NSUTF8StringEncoding];
  NSMenuItem *item = find_item((__bridge NSMenu *)pointer, title);
  if (!item)
    return -1;
  switch (field) {
  case 0:
    return item.state;
  case 1:
    return (int64_t)item.image.representations.count;
  case 2:
    return (int64_t)item.menu.numberOfItems;
  default:
    return -2;
  }
}

void scoop_test_menu_observe(void) {
  tracking_observer = [NSNotificationCenter.defaultCenter
      addObserverForName:NSMenuDidBeginTrackingNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification *notification) {
                tracking_menu = notification.object;
              }];
}

int32_t scoop_test_menu_tracking(void) { return tracking_menu != nil; }

int32_t scoop_test_popup_action(const uint8_t *text, int64_t length) {
  NSString *title = [[NSString alloc] initWithBytes:text
                                             length:(NSUInteger)length
                                           encoding:NSUTF8StringEncoding];
  NSMenuItem *item = find_item(tracking_menu, title);
  BOOL result = item && [NSApp sendAction:item.action to:item.target from:item];
  [tracking_menu cancelTrackingWithoutAnimation];
  tracking_menu = nil;
  return result;
}

void scoop_test_menu_end(void) {
  [NSNotificationCenter.defaultCenter removeObserver:tracking_observer];
  tracking_observer = nil;
  tracking_menu = nil;
}
