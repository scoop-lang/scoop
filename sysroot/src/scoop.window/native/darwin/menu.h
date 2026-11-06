#ifndef SCOOP_WINDOW_DARWIN_MENU_H
#define SCOOP_WINDOW_DARWIN_MENU_H

#import "window.h"

typedef struct {
  int32_t key, modifiers;
  int64_t error;
} SWMenuKey;

@class SWMenu;
@interface SWMenuEntry : NSObject
@property(nonatomic, weak) NSMenuItem *item;
@property(nonatomic) int64_t identity;
@property(nonatomic) int32_t kind, role;
@property(nonatomic) BOOL enabled, live;
@end

@interface SWMenu : NSObject <NSMenuDelegate>
@property(nonatomic, weak) SWApplication *application;
@property(nonatomic, weak) SWWindow *window;
@property(nonatomic, strong) NSMenu *menu;
@property(nonatomic, strong) NSMenu *previousMenu;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWMenuEntry *> *entries;
@property(nonatomic) int64_t identity, serial, selected;
@property(nonatomic) int32_t binding;
@property(nonatomic) NSUInteger tracking;
@property(nonatomic) BOOL externalTracking, presenting, cancelled;
@end

@interface SWMenu (Actions) <NSMenuItemValidation>
- (void)activate:(NSMenuItem *)item;
@end

void sw_menu_event(SWMenu *menu, int32_t kind, int64_t item, int64_t window);
NSEventModifierFlags sw_menu_modifiers(int32_t value);
BOOL sw_menu_activate(SWMenu *menu, SWMenuEntry *entry);
NSString *sw_menu_key_string(int32_t key);

#endif
