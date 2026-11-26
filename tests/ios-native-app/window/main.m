@import ScoopProbe;
@import UIKit;
@import QuartzCore;
#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

static ScoopWindowApi api;
static __weak UIWindowScene *currentScene;
static unsigned events[8];
static BOOL disconnecting;

static void inspect(void *nativeWindow, void *nativeView,
                    ScoopWindowMetrics metrics) {
  UIWindow *window = (__bridge UIWindow *)nativeWindow;
  UIView *view = (__bridge UIView *)nativeView;
  assert(NSThread.isMainThread && window.windowScene == currentScene);
  assert(window.rootViewController.view == view &&
         [view.layer isKindOfClass:CAMetalLayer.class]);
  assert(view.multipleTouchEnabled && !window.hidden);
  assert(view.bounds.size.width == metrics.width &&
         view.bounds.size.height == metrics.height);
  assert(view.traitCollection.displayScale == metrics.scale &&
         metrics.scale > 0);
  assert(metrics.pixelsWide == ceil(metrics.width * metrics.scale));
  assert(metrics.pixelsHigh == ceil(metrics.height * metrics.scale));
  assert(UIEdgeInsetsEqualToEdgeInsets(
      view.safeAreaInsets, UIEdgeInsetsMake(metrics.top, metrics.left,
                                            metrics.bottom, metrics.right)));
}

static void event(int32_t kind) {
  assert(NSThread.isMainThread && kind >= 1 && kind <= 7);
  events[kind]++;
  if (!disconnecting && events[1] && events[2] && events[3] && events[4]) {
    disconnecting = YES;
    dispatch_async(dispatch_get_main_queue(), ^{
      assert(events[1] == 1 && events[2] == 1 && events[3] == 1 &&
             events[4] == 1);
      void *scene = (__bridge void *)currentScene;
      assert(api.control(api.context, 3, scene));
      assert(events[5] == 1 && events[6] == 0);
      dispatch_after(
          dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_MSEC),
          dispatch_get_main_queue(), ^{
            assert(events[6] == 0 &&
                   api.control(api.context, 4, (__bridge void *)currentScene));
          });
    });
  }
  if (kind == 6) {
    assert(events[5] == 1 && events[7] == 1);
    dispatch_async(dispatch_get_main_queue(), ^{
      assert(api.control(api.context, 5, (__bridge void *)currentScene));
      dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 30 * NSEC_PER_MSEC),
                     dispatch_get_main_queue(), ^{
                       puts("scoop-ios-window-ok");
                       fflush(stdout);
#if TARGET_OS_SIMULATOR
                       exit(0);
#endif
                     });
    });
  }
}

@interface WindowAppDelegate : UIResponder <UIApplicationDelegate>
@end
@implementation WindowAppDelegate
- (BOOL)application:(UIApplication *)application
    didFinishLaunchingWithOptions:(NSDictionary *)options {
  ScoopWindowHost host = {inspect, event};
  ScoopAppError error = {0};
  assert(scoop_app_init(&host, &api, &error) == SCOOP_APP_INIT_OK);
  return YES;
}
@end

@interface WindowProbeScene : UIResponder <UIWindowSceneDelegate>
@end
@implementation WindowProbeScene
- (void)scene:(UIScene *)scene
    willConnectToSession:(UISceneSession *)session
                 options:(UISceneConnectionOptions *)options {
  currentScene = (UIWindowScene *)scene;
  assert(api.control(api.context, 1, (__bridge void *)scene));
}
- (void)sceneDidBecomeActive:(UIScene *)scene {
  dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
    assert(api.control(api.context, 2, (__bridge void *)scene));
  });
}
- (void)sceneDidDisconnect:(UIScene *)scene {
  assert(api.control(api.context, 3, (__bridge void *)scene));
}
@end

int main(int argc, char **argv) {
  @autoreleasepool {
    return UIApplicationMain(argc, argv, nil,
                             NSStringFromClass(WindowAppDelegate.class));
  }
}
