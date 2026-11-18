#import "../../native/darwin/menu.h"
#include "../../native/text_services.h"

extern int64_t scoop_window_menu_bind(void *, int32_t, int64_t, void *);
extern int64_t scoop_window_menu_unbind(void *);
extern SWTextServiceString scoop_window_menu_event_text(void *);

static NSMenu *original_services_parent;

int32_t scoop_test_menu_services_begin(void *pointer, void *target) {
  SWMenu *menu = (__bridge SWMenu *)pointer;
  if (scoop_window_menu_bind(pointer, 2, 1, target))
    return 1;
  if (NSApp.servicesMenu && !NSApp.servicesMenu.supermenu) {
    original_services_parent = [[NSMenu alloc] initWithTitle:@"Original host"];
    NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:@"Services"
                                                  action:nil
                                           keyEquivalent:@""];
    [original_services_parent addItem:item];
    item.submenu = NSApp.servicesMenu;
  }
  // Drive the public requestor callbacks without entering an OS tracking loop.
  menu.presenting = YES;
  return (int32_t)sw_menu_begin_text_services(menu);
}

int32_t scoop_test_menu_services_probe(void *pointer, int32_t mode) {
  SWMenu *menu = (__bridge SWMenu *)pointer;
  id delegate = NSApp.delegate;
  id<NSServicesMenuRequestor> requestor =
      [delegate validRequestorForSendType:NSPasteboardTypeString
                               returnType:nil];
  if (mode == 3) {
    NSPasteboard *late = NSPasteboard.pasteboardWithUniqueName;
    @try {
      [late declareTypes:@[ NSPasteboardTypeString ] owner:nil];
      [late setString:@"late replacement" forType:NSPasteboardTypeString];
      return requestor || [menu readSelectionFromPasteboard:late] ||
             [menu writeSelectionToPasteboard:late
                                        types:@[ NSPasteboardTypeString ]];
    } @finally {
      [late releaseGlobally];
    }
  }
  if (!requestor || NSApp.servicesMenu != menu.textServices.item.submenu)
    return 2;
  if ([delegate validRequestorForSendType:NSPasteboardTypePNG returnType:nil] ||
      [delegate validRequestorForSendType:NSPasteboardTypeString
                               returnType:NSPasteboardTypePNG])
    return 3;
  BOOL returnsText =
      [delegate validRequestorForSendType:NSPasteboardTypeString
                               returnType:NSPasteboardTypeString] != nil;
  if (returnsText != (mode != 2))
    return 4;
  NSPasteboard *pasteboard = NSPasteboard.pasteboardWithUniqueName;
  NSInteger clipboardChange = NSPasteboard.generalPasteboard.changeCount;
  @try {
    if (![requestor writeSelectionToPasteboard:pasteboard
                                         types:@[ NSPasteboardTypeString ]] ||
        ![[pasteboard stringForType:NSPasteboardTypeString]
            isEqualToString:@"😀 words"])
      return 5;
    NSString *replacement = mode == 1 ? @"" : @"替换😀";
    [pasteboard setString:replacement forType:NSPasteboardTypeString];
    if ([requestor readSelectionFromPasteboard:pasteboard] != returnsText)
      return 6;
    NSUInteger before = menu.application.events.count;
    NSMenu *submenu = menu.textServices.item.submenu;
    NSMenuItem *action = [[NSMenuItem alloc] initWithTitle:@"Test service"
                                                    action:nil
                                             keyEquivalent:@""];
    [NSNotificationCenter.defaultCenter
        postNotificationName:NSMenuDidSendActionNotification
                      object:submenu
                    userInfo:@{@"MenuItem" : action}];
    [NSNotificationCenter.defaultCenter
        postNotificationName:NSMenuDidSendActionNotification
                      object:submenu
                    userInfo:@{@"MenuItem" : action}];
    if (menu.application.events.count != before + 1)
      return 7;
    SWEvent *event = menu.application.events.lastObject;
    if (event.header.kind != 110 ||
        event.data.b != menu.textServices.identity ||
        ((event.data.flags & 16) != 0) != returnsText ||
        menu.selected != menu.textServices.identity)
      return 8;
    // A queued result remains owned by the event after the menu snapshot
    // changes.
    menu.textServices.serviceResult = nil;
    SWTextServiceString result =
        scoop_window_menu_event_text((__bridge void *)event);
    NSString *text = [[NSString alloc] initWithBytes:result.bytes
                                              length:(NSUInteger)result.length
                                            encoding:NSUTF8StringEncoding];
    if (returnsText && ![text isEqualToString:replacement])
      return 9;
    if (NSPasteboard.generalPasteboard.changeCount != clipboardChange)
      return 10;
    return 0;
  } @finally {
    [pasteboard releaseGlobally];
  }
}

int32_t scoop_test_menu_services_end(void *pointer) {
  SWMenu *menu = (__bridge SWMenu *)pointer;
  NSMenu *previous = menu.previousServicesParent;
  NSMenuItem *previous_item = menu.previousServicesItem;
  NSMenu *popup = menu.textServices.item.submenu;
  sw_menu_end_text_services(menu);
  menu.presenting = NO;
  int64_t status = scoop_window_menu_unbind(pointer);
  BOOL restored =
      previous ? previous_item.submenu == popup && popup.supermenu == previous
               : popup.supermenu == nil;
  if (previous == original_services_parent)
    previous_item.submenu = nil;
  [original_services_parent removeAllItems];
  original_services_parent = nil;
  return status                               ? 1
         : !restored                          ? 2
         : menu.application.textServicesPopup ? 3
                                              : 0;
}
