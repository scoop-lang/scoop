#import "text.h"
#include <math.h>

@implementation SWIScene {
  CGRect _bounds;
  UIEdgeInsets _insets;
  CGFloat _scale;
}
- (SWISnapshot)snapshot {
  SWIView *view = self.view;
  CGRect bounds = view.bounds;
  CGFloat scale = view.traitCollection.displayScale;
  if (scale <= 0)
    scale = view.contentScaleFactor;
  UIEdgeInsets insets = view.safeAreaInsets;
#if __IPHONE_OS_VERSION_MAX_ALLOWED >= 260000
  if (@available(iOS 26.0, *)) {
    if (view.traitCollection.userInterfaceIdiom == UIUserInterfaceIdiomPad)
      insets = [view edgeInsetsForLayoutRegion:
                         [UIViewLayoutRegion
                             safeAreaLayoutRegionWithCornerAdaptation:
                                 UIViewLayoutRegionAdaptivityAxisVertical]];
  }
#endif
  return (SWISnapshot){bounds.size.width,
                       bounds.size.height,
                       scale,
                       insets.top,
                       insets.left,
                       insets.bottom,
                       insets.right,
                       self.foreground && !self.window.hidden,
                       self.active && self.window.isKeyWindow,
                       !self.foreground,
                       view.traitCollection.userInterfaceStyle ==
                           UIUserInterfaceStyleDark};
}
- (void)emit:(int32_t)kind flags:(int32_t)flags {
  SWIEvent *event = [SWIEvent new];
  event.header = (SWEventHeader){.window = self.identity, .kind = kind};
  event.data = (SWEventData){.flags = flags};
  event.snapshot = self.snapshot;
  [self.application enqueue:event];
}
- (void)geometryChanged {
  SWISnapshot value = self.snapshot;
  UIEdgeInsets edges =
      UIEdgeInsetsMake(value.top, value.left, value.bottom, value.right);
  if (CGRectEqualToRect(_bounds, self.view.bounds) && _scale == value.scale &&
      UIEdgeInsetsEqualToEdgeInsets(_insets, edges))
    return;
  _bounds = self.view.bounds;
  _scale = value.scale;
  _insets = edges;
  [self emit:2 flags:0];
}
- (void)activation:(NSNotification *)note {
  BOOL next = [note.name isEqualToString:UISceneDidActivateNotification];
  if (self.active == next)
    return;
  if (!next) {
    [self.view cancelContacts:SWPointerFocusLost];
    [self cancelKeys];
  }
  self.active = next;
  [self emit:(next ? 101 : 102) flags:next];
  [self emit:4 flags:self.snapshot.focused];
}
- (void)foreground:(NSNotification *)note {
  self.foreground =
      [note.name isEqualToString:UISceneWillEnterForegroundNotification];
  [self emit:5 flags:!self.foreground];
  if (self.foreground)
    [self geometryChanged];
}
- (void)focus:(NSNotification *)note {
  if (!self.window.isKeyWindow) {
    [self.view cancelContacts:SWPointerFocusLost];
    [self cancelKeys];
  }
  [self emit:4 flags:self.snapshot.focused];
}
- (void)preferencesChanged {
  [self emit:104 flags:0];
}
- (void)preferencesNotification:(NSNotification *)note {
  [self preferencesChanged];
}
@end

void *scoop_window_ios_scene_create(void *application, void *scenePointer,
                                    int64_t identity) {
  UIWindowScene *scene = (__bridge UIWindowScene *)scenePointer;
  if (![scene isKindOfClass:UIWindowScene.class] || !NSThread.isMainThread)
    return NULL;
  SWIScene *record = [SWIScene new];
  SWIApplication *app = (__bridge SWIApplication *)application;
  record.application = app;
  record.identity = identity;
  record.foreground =
      scene.activationState == UISceneActivationStateForegroundActive ||
      scene.activationState == UISceneActivationStateForegroundInactive;
  record.active =
      scene.activationState == UISceneActivationStateForegroundActive;
  record.window = [[UIWindow alloc] initWithWindowScene:scene];
  UIViewController *controller = [UIViewController new];
  record.view = [[SWIView alloc] initWithFrame:record.window.bounds];
  // Give the scene a visible, interactive surface before its first Metal
  // drawable.
  record.view.backgroundColor = UIColor.systemBackgroundColor;
  record.view.multipleTouchEnabled = YES;
  record.view.owner = record;
  controller.view = record.view;
  record.window.rootViewController = controller;
  app.preferenceView = record.view;
  __weak SWIScene *weakScene = record;
  [record.view
      registerForTraitChanges:@[
        UITraitDisplayScale.class, UITraitUserInterfaceStyle.class,
        UITraitPreferredContentSizeCategory.class,
        UITraitAccessibilityContrast.class
      ]
                  withHandler:^(UIView *view, UITraitCollection *previous) {
                    SWIScene *owner = weakScene;
                    [owner geometryChanged];
                    if (previous.userInterfaceStyle !=
                        view.traitCollection.userInterfaceStyle)
                      [owner emit:6 flags:owner.snapshot.dark];
                    [owner preferencesChanged];
                  }];
  NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
  for (NSNotificationName name in
       @[ UISceneDidActivateNotification, UISceneWillDeactivateNotification ])
    [center addObserver:record
               selector:@selector(activation:)
                   name:name
                 object:scene];
  for (NSNotificationName name in @[
         UISceneWillEnterForegroundNotification,
         UISceneDidEnterBackgroundNotification
       ])
    [center addObserver:record
               selector:@selector(foreground:)
                   name:name
                 object:scene];
  for (NSNotificationName name in
       @[ UIWindowDidBecomeKeyNotification, UIWindowDidResignKeyNotification ])
    [center addObserver:record
               selector:@selector(focus:)
                   name:name
                 object:record.window];
  for (NSNotificationName name in @[
         UIAccessibilityReduceMotionStatusDidChangeNotification,
         UIAccessibilityReduceTransparencyStatusDidChangeNotification
       ])
    [center addObserver:record
               selector:@selector(preferencesNotification:)
                   name:name
                 object:nil];
  return (__bridge_retained void *)record;
}
void scoop_window_ios_scene_show(void *pointer) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  [scene.window makeKeyAndVisible];
  [scene.view becomeFirstResponder];
  [scene.view layoutIfNeeded];
  [scene geometryChanged];
}
void scoop_window_ios_scene_destroy(void *pointer) {
  SWIScene *scene = (__bridge_transfer SWIScene *)pointer;
  [NSNotificationCenter.defaultCenter removeObserver:scene];
  [scene.view.editor closeConnection];
  scene.application = nil;
  scene.view.owner = nil;
  scene.window.hidden = YES;
  scene.window.rootViewController = nil;
}
SWISnapshot scoop_window_ios_snapshot(void *pointer) {
  return ((__bridge SWIScene *)pointer).snapshot;
}
void *scoop_window_ios_uiwindow(void *pointer) {
  return (__bridge void *)((__bridge SWIScene *)pointer).window;
}
void *scoop_window_ios_view(void *pointer) {
  return (__bridge void *)((__bridge SWIScene *)pointer).view;
}
