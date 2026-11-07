#import "menu.h"

static SEL role_selector(int32_t role) {
  switch (role) {
  case 4:
    return @selector(cut:);
  case 5:
    return @selector(copy:);
  case 6:
    return @selector(paste:);
  case 7:
    return @selector(selectAll:);
  case 8:
    return @selector(undo:);
  case 9:
    return @selector(redo:);
  default:
    return NULL;
  }
}

static NSWindow *target_window(SWMenu *menu) {
  if (menu.binding == 1 || menu.binding == 2)
    return menu.window.window;
  return NSApp.keyWindow;
}

static SWWindow *scoop_target(SWMenu *menu, NSWindow *target) {
  for (SWWindow *window in menu.application.windows.allValues)
    if (window.window == target)
      return window;
  return nil;
}

static BOOL can_activate(SWMenu *menu, SWMenuEntry *entry) {
  if (!entry.live || entry.kind >= 3 || menu.binding < 0 || menu.cancelled)
    return NO;
  NSWindow *target = target_window(menu);
  SWWindow *window = scoop_target(menu, target);
  if (entry.role != 1 && window && (window.inputBlocked || window.destroying))
    return NO;
  if (entry.role >= 4) {
    if (window && target.firstResponder == window.view)
      return entry.enabled && window.view.imeAllowed;
    SEL selector = role_selector(entry.role);
    id responder = [NSApp targetForAction:selector to:nil from:entry.item];
    if (!responder)
      return NO;
    // Native fields validate the actual edit target, independently of Scoop's
    // snapshot.
    if ([responder respondsToSelector:@selector(validateUserInterfaceItem:)]) {
      NSMenuItem *validation =
          [[NSMenuItem alloc] initWithTitle:entry.item.title
                                     action:selector
                              keyEquivalent:@""];
      return [responder validateUserInterfaceItem:validation];
    }
    return YES;
  }
  if (!entry.enabled)
    return NO;
  if (entry.role == 2)
    return target && (target.styleMask & NSWindowStyleMaskMiniaturizable);
  if (entry.role == 3)
    return target && (target.styleMask & NSWindowStyleMaskResizable);
  return YES;
}

BOOL sw_menu_activate(SWMenu *menu, SWMenuEntry *entry) {
  if (!can_activate(menu, entry))
    return NO;
  NSWindow *target = target_window(menu);
  SWWindow *window = scoop_target(menu, target);
  menu.selected = entry.identity;
  if (entry.role == 0) {
    sw_menu_event(menu, 110, entry.identity, window ? window.identity : 0);
  } else if (entry.role == 1) {
    SWEvent *event = [SWEvent new];
    SWEventData data = {0};
    data.kind = 101;
    data.flags = 1;
    event.data = data;
    [menu.application enqueue:event];
  } else if (entry.role == 2) {
    [target miniaturize:nil];
  } else if (entry.role == 3) {
    [target zoom:nil];
  } else if (window && target.firstResponder == window.view) {
    SWEvent *event = sw_event(window, 16);
    SWEventData data = event.data;
    data.a = entry.role;
    event.data = data;
    [menu.application enqueue:event];
  } else {
    [NSApp sendAction:role_selector(entry.role) to:nil from:entry.item];
  }
  return YES;
}

int32_t scoop_window_menu_activate(void *pointer, int64_t identity) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  SWMenuEntry *entry = owner.entries[@(identity)];
  return entry && sw_menu_activate(owner, entry);
}

BOOL sw_menu_key_equivalent(SWApplication *application, NSEvent *event) {
  return application.menuBar &&
         [application.menuBar.menu performKeyEquivalent:event];
}

NSEventModifierFlags sw_menu_modifiers(int32_t value) {
  return ((value & 1) ? NSEventModifierFlagControl : 0) |
         ((value & 2) ? NSEventModifierFlagOption : 0) |
         ((value & 4) ? NSEventModifierFlagShift : 0) |
         ((value & 8) ? NSEventModifierFlagCommand : 0);
}

NSString *sw_menu_key_string(int32_t key) {
  if (!key)
    return @"";
  UTF32Char scalar = (UTF32Char)key;
  return [[NSString alloc] initWithBytes:&scalar
                                  length:sizeof(scalar)
                                encoding:NSUTF32LittleEndianStringEncoding];
}

SWMenuKey scoop_window_menu_resolve_key(int32_t kind, int32_t key,
                                        int32_t modifiers) {
  if (modifiers & 16)
    return (SWMenuKey){.error = -1};
  if (kind) {
    static const unichar named[] = {0,
                                    '\r',
                                    '\t',
                                    ' ',
                                    NSBackspaceCharacter,
                                    0x1b,
                                    NSDeleteFunctionKey,
                                    NSLeftArrowFunctionKey,
                                    NSRightArrowFunctionKey,
                                    NSUpArrowFunctionKey,
                                    NSDownArrowFunctionKey,
                                    NSHomeFunctionKey,
                                    NSEndFunctionKey,
                                    NSPageUpFunctionKey,
                                    NSPageDownFunctionKey};
    if (key >= 21 && key <= 40)
      key = NSF1FunctionKey + key - 21;
    else if (key > 0 && key < (int32_t)(sizeof(named) / sizeof(named[0])))
      key = named[key];
    else
      return (SWMenuKey){.error = -1};
  } else {
    NSString *text = [sw_menu_key_string(key) lowercaseString];
    NSData *scalar = [text dataUsingEncoding:NSUTF32LittleEndianStringEncoding];
    if (scalar.length != sizeof(UTF32Char))
      return (SWMenuKey){.error = -1};
    memcpy(&key, scalar.bytes, sizeof(key));
  }
  return (SWMenuKey){key, modifiers, 0};
}

@implementation SWMenu (Actions)
- (void)activate:(NSMenuItem *)item {
  SWMenuEntry *entry = item.representedObject;
  if (entry)
    sw_menu_activate(self, entry);
}
- (BOOL)validateMenuItem:(NSMenuItem *)item {
  return can_activate(self, item.representedObject);
}
@end
