#import "menu.h"

void sw_menu_event(SWMenu *menu, int32_t kind, int64_t item, int64_t window) {
  SWEvent *event = [SWEvent new];
  SWEventData data = {0};
  data.kind = kind;
  data.flags = menu.binding;
  data.a = menu.identity;
  data.b = item;
  data.c = menu.serial;
  data.d = window;
  event.data = data;
  [menu.application enqueue:event];
}

@implementation SWMenu
- (void)menuWillOpen:(NSMenu *)menu {
  self.tracking += 1;
}
- (void)menuDidClose:(NSMenu *)menu {
  if (self.tracking)
    self.tracking -= 1;
  if (!self.tracking)
    sw_menu_event(self, 112, 0, 0);
}
@end

int32_t scoop_window_menu_busy(void *pointer) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  return owner.tracking || owner.externalTracking || owner.presenting;
}

void scoop_window_menu_external_tracking(void *pointer, int32_t tracking) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  if (tracking)
    owner.selected = 0;
  owner.externalTracking = tracking != 0;
  if (!tracking)
    sw_menu_event(owner, 112, 0, 0);
}

int64_t scoop_window_menu_selection(void *pointer) {
  return ((__bridge SWMenu *)pointer).selected;
}

int64_t scoop_window_menu_bind(void *pointer, int32_t kind, int64_t serial,
                               void *window) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  if (kind == 1)
    return -1;
  @try {
    owner.binding = kind;
    owner.cancelled = NO;
    owner.serial = serial;
    owner.window = (__bridge SWWindow *)window;
    if (kind == 0) {
      owner.previousMenu = NSApp.mainMenu;
      NSApp.mainMenu = owner.menu;
      owner.application.menuBar = owner;
    }
    return 0;
  } @catch (NSException *exception) {
    owner.binding = -1;
    owner.window = nil;
    return 1;
  }
}

int64_t scoop_window_menu_unbind(void *pointer) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  @try {
    if (owner.binding == 0) {
      if (NSApp.mainMenu == owner.menu)
        NSApp.mainMenu = owner.previousMenu;
      if (owner.application.menuBar == owner)
        owner.application.menuBar = nil;
    }
    owner.previousMenu = nil;
    owner.binding = -1;
    owner.window = nil;
    owner.serial = 0;
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

void scoop_window_menu_cancel(void *pointer) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  owner.cancelled = YES;
  [owner.menu cancelTrackingWithoutAnimation];
  for (SWMenuEntry *entry in owner.entries.allValues)
    [entry.item.submenu cancelTrackingWithoutAnimation];
}

int64_t scoop_window_menu_popup(void *pointer, void *target, double x, double y,
                                double width, double height, int32_t anchor) {
  @autoreleasepool {
    SWMenu *owner = (__bridge SWMenu *)pointer;
    SWWindow *window = (__bridge SWWindow *)target;
    owner.presenting = YES;
    owner.selected = 0;
    int64_t status = 0;
    @try {
      [owner.menu
          popUpMenuPositioningItem:nil
                        atLocation:NSMakePoint(x, y + (anchor ? height : 0))
                            inView:window.view];
    } @catch (NSException *exception) {
      status = 1;
    } @finally {
      owner.presenting = NO;
      owner.tracking = 0;
    }
    if (!status)
      sw_menu_event(owner, 111, owner.selected, window.identity);
    return status;
  }
}
