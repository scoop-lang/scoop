#import "edit_actions.h"

@interface SWIEditMenu : NSObject <UIEditMenuInteractionDelegate>
@property(nonatomic, weak) SWIView *view;
@property(nonatomic, strong) UIEditMenuInteraction *interaction;
@property(nonatomic, strong) NSMutableArray<NSNumber *> *roles;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, NSString *> *titles;
@property(nonatomic) CGRect anchor;
@property(nonatomic) int flags;
@property(nonatomic) BOOL systemTitles, visible, selected;
@property(nonatomic) SWICallback callback;
@property(nonatomic) void *context;
@end

static void systemTitles(NSArray<UIMenuElement *> *elements,
                         NSMutableDictionary *titles) {
  for (UIMenuElement *item in elements) {
    if ([item isKindOfClass:UIMenu.class])
      systemTitles(((UIMenu *)item).children, titles);
    if ([item isKindOfClass:UICommand.class]) {
      SWIEditAction action = swi_edit_action(((UICommand *)item).action);
      if (action && item.title.length)
        titles[@(action)] = item.title;
    }
  }
}
@implementation SWIEditMenu
- (UIMenu *)editMenuInteraction:(UIEditMenuInteraction *)interaction
           menuForConfiguration:(UIEditMenuConfiguration *)configuration
               suggestedActions:(NSArray<UIMenuElement *> *)suggestedActions {
  NSMutableDictionary *titles = [self.titles mutableCopy];
  if (self.systemTitles)
    systemTitles(suggestedActions, titles);
  NSMutableArray *actions = [NSMutableArray array];
  __weak SWIEditMenu *weakMenu = self;
  for (NSNumber *role in self.roles) {
    int code = role.intValue;
    if (!(self.flags & (1 << (code - 4))))
      continue;
    UIAction *action =
        [UIAction actionWithTitle:titles[role]
                            image:nil
                       identifier:nil
                          handler:^(UIAction *sender) {
                            SWIEditMenu *menu = weakMenu;
                            if (!menu.callback || menu.selected ||
                                !(menu.flags & (1 << (code - 4))))
                              return;
                            menu.selected = YES;
                            menu.callback(menu.context, code);
                          }];
    [actions addObject:action];
  }
  return [UIMenu menuWithChildren:actions];
}
- (CGRect)editMenuInteraction:(UIEditMenuInteraction *)interaction
    targetRectForConfiguration:(UIEditMenuConfiguration *)configuration {
  return self.anchor;
}
- (void)editMenuInteraction:(UIEditMenuInteraction *)interaction
    willDismissMenuForConfiguration:(UIEditMenuConfiguration *)configuration
                           animator:
                               (id<UIEditMenuInteractionAnimating>)animator {
  [animator addCompletion:^{
    self.visible = NO;
  }];
}
@end

void *scoop_window_ios_edit_menu_create(void *pointer, SWICallback callback,
                                        void *context, double x, double y,
                                        double width, double height,
                                        int32_t flags, int32_t system) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  if (!scene.window.isKeyWindow || !scene.active)
    return NULL;
  SWIEditMenu *menu = [SWIEditMenu new];
  menu.view = scene.view;
  menu.roles = [NSMutableArray array];
  menu.titles = [NSMutableDictionary dictionary];
  menu.callback = callback;
  menu.context = context;
  menu.flags = flags;
  menu.systemTitles = system != 0;
  menu.anchor = CGRectMake(x, y, width, height);
  menu.interaction = [[UIEditMenuInteraction alloc] initWithDelegate:menu];
  [menu.view addInteraction:menu.interaction];
  return (__bridge_retained void *)menu;
}
void scoop_window_ios_edit_menu_add(void *pointer, int role,
                                    const uint8_t *label, int64_t length) {
  SWIEditMenu *menu = (__bridge SWIEditMenu *)pointer;
  [menu.roles addObject:@(role)];
  menu.titles[@(role)] = [[NSString alloc] initWithBytes:label
                                                  length:(NSUInteger)length
                                                encoding:NSUTF8StringEncoding];
}
void scoop_window_ios_edit_menu_present(void *pointer) {
  SWIEditMenu *menu = (__bridge SWIEditMenu *)pointer;
  CGPoint origin =
      CGPointMake(CGRectGetMidX(menu.anchor), CGRectGetMidY(menu.anchor));
  menu.visible = YES;
  [menu.interaction
      presentEditMenuWithConfiguration:[UIEditMenuConfiguration
                                           configurationWithIdentifier:nil
                                                           sourcePoint:origin]];
}
int32_t scoop_window_ios_edit_menu_pending(void *pointer) {
  SWIEditMenu *menu = (__bridge SWIEditMenu *)pointer;
  return menu.visible || menu.selected;
}
void scoop_window_ios_edit_menu_update(void *pointer, int32_t flags) {
  SWIEditMenu *menu = (__bridge SWIEditMenu *)pointer;
  if (menu.flags != flags) {
    menu.flags = flags;
    [menu.interaction reloadVisibleMenu];
  }
}
void scoop_window_ios_edit_menu_close(void *pointer) {
  SWIEditMenu *menu = CFBridgingRelease(pointer);
  menu.callback = NULL;
  [menu.interaction dismissMenu];
  [menu.view removeInteraction:menu.interaction];
}
