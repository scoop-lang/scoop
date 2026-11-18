#include "../text_services.h"
#import "menu.h"
#include <errno.h>

// Services declarations can use either public plain-text pasteboard spelling.
static BOOL text_type(NSPasteboardType type) {
  return [type isEqualToString:NSPasteboardTypeString] ||
         [type isEqualToString:@"NSStringPboardType"];
}

int64_t scoop_window_menu_text_services(void *pointer, int64_t identity,
                                        const uint8_t *bytes, int64_t length,
                                        int32_t accept_replacement) {
  SWMenu *menu = (__bridge SWMenu *)pointer;
  SWMenuEntry *entry = menu.entries[@(identity)];
  if (!entry || entry.kind != 5)
    return EINVAL;
  NSString *text = [[NSString alloc] initWithBytes:bytes
                                            length:(NSUInteger)length
                                          encoding:NSUTF8StringEncoding];
  if (!text)
    return EINVAL;
  entry.serviceText = text;
  entry.serviceReturnsText = accept_replacement != 0;
  return 0;
}

SWTextServiceString scoop_window_menu_event_text(void *pointer) {
  NSData *text = ((__bridge SWEvent *)pointer).text;
  return (SWTextServiceString){text.length ? text.bytes : (const uint8_t *)"",
                               (int64_t)text.length};
}

int64_t sw_menu_begin_text_services(SWMenu *menu) {
  SWMenuEntry *entry = menu.textServices;
  if (!entry)
    return 0;
  if (menu.application.textServicesPopup)
    return -5;
  menu.application.textServicesPopup = menu;
  entry.serviceResult = nil;
  entry.serviceInvoked = NO;
  NSArray *types = @[ NSPasteboardTypeString, @"NSStringPboardType" ];
  [NSApp registerServicesMenuSendTypes:types returnTypes:types];
  // AppKit retains the first registered Services menu. Borrow that actual tree
  // instead of repeatedly assigning newly constructed popup submenus.
  NSMenu *services = NSApp.servicesMenu;
  if (!services) {
    NSApp.servicesMenu = [[NSMenu alloc] initWithTitle:entry.item.title];
    services = NSApp.servicesMenu;
  }
  NSMenu *parent = services.supermenu;
  if (parent) {
    NSInteger index = [parent indexOfItemWithSubmenu:services];
    if (index >= 0) {
      menu.previousServicesParent = parent;
      menu.previousServicesItem = [parent itemAtIndex:index];
      menu.previousServicesItem.submenu = nil;
    }
  }
  entry.item.submenu = services;
  [NSNotificationCenter.defaultCenter
      addObserver:menu
         selector:@selector(textServiceAction:)
             name:NSMenuDidSendActionNotification
           object:nil];
  return 0;
}

void sw_menu_end_text_services(SWMenu *menu) {
  [NSNotificationCenter.defaultCenter
      removeObserver:menu
                name:NSMenuDidSendActionNotification
              object:nil];
  if (menu.application.textServicesPopup != menu)
    return;
  NSMenu *services = menu.textServices.item.submenu;
  menu.textServices.item.submenu = nil;
  NSMenuItem *previous = menu.previousServicesItem;
  if (menu.previousServicesParent &&
      previous.menu == menu.previousServicesParent && !previous.submenu)
    previous.submenu = services;
  menu.application.textServicesPopup = nil;
  menu.previousServicesItem = nil;
  menu.previousServicesParent = nil;
}

@implementation SWMenu (TextServices)
- (BOOL)textServicesAvailable {
  SWMenuEntry *entry = self.textServices;
  SWWindow *window = self.window;
  return self.application.textServicesPopup == self && self.presenting &&
         self.binding == 2 && !self.cancelled && entry.live && entry.enabled &&
         entry.serviceText && !entry.serviceInvoked && window &&
         !window.destroying && !window.inputBlocked;
}

- (id)textServiceRequestorForSendType:(NSPasteboardType)sendType
                           returnType:(NSPasteboardType)returnType {
  if (![self textServicesAvailable] || (!sendType && !returnType) ||
      (sendType && !text_type(sendType)) ||
      (returnType &&
       (!text_type(returnType) || !self.textServices.serviceReturnsText)))
    return nil;
  return self;
}

- (BOOL)writeSelectionToPasteboard:(NSPasteboard *)pasteboard
                             types:(NSArray<NSPasteboardType> *)types {
  if (![self textServicesAvailable])
    return NO;
  NSMutableArray *accepted = [NSMutableArray new];
  for (NSPasteboardType type in types)
    if (text_type(type))
      [accepted addObject:type];
  if (!accepted.count)
    return NO;
  [pasteboard declareTypes:accepted owner:nil];
  for (NSPasteboardType type in accepted)
    if (![pasteboard setString:self.textServices.serviceText forType:type])
      return NO;
  return YES;
}

- (BOOL)readSelectionFromPasteboard:(NSPasteboard *)pasteboard {
  if (![self textServicesAvailable] || !self.textServices.serviceReturnsText)
    return NO;
  NSPasteboardType type = [pasteboard availableTypeFromArray:@[
    NSPasteboardTypeString, @"NSStringPboardType"
  ]];
  NSString *text = type ? [pasteboard stringForType:type] : nil;
  if (!text)
    return NO;
  self.textServices.serviceResult =
      [text dataUsingEncoding:NSUTF8StringEncoding];
  return self.textServices.serviceResult != nil;
}

- (void)textServiceAction:(NSNotification *)notification {
  if (![self textServicesAvailable])
    return;
  NSMenu *submenu = notification.object;
  while (submenu && submenu != self.textServices.item.submenu)
    submenu = submenu.supermenu;
  if (!submenu)
    return;
  SWMenuEntry *entry = self.textServices;
  entry.serviceInvoked = YES;
  self.selected = entry.identity;
  SWEvent *event = [SWEvent new];
  event.header = (SWEventHeader){.kind = 110};
  event.data =
      (SWEventData){.flags = self.binding | (entry.serviceResult ? 16 : 0),
                    .a = self.identity,
                    .b = entry.identity,
                    .c = self.serial,
                    .d = self.window.identity};
  event.text = entry.serviceResult;
  [self.application enqueue:event];
}
@end

@implementation SWApplication (TextServices)
- (id)validRequestorForSendType:(NSPasteboardType)sendType
                     returnType:(NSPasteboardType)returnType {
  return [self.textServicesPopup textServiceRequestorForSendType:sendType
                                                      returnType:returnType];
}
@end
