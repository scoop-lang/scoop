@import ScoopProbe;
@import UIKit;
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

static ScoopSurfaceApi api;
static __weak UIWindowScene *scene;
static BOOL observe, contentClosed;

static void report(int32_t stage) {
  assert(NSThread.isMainThread);
  if (stage == 2 || stage == 5) {
    if (observe) {
      UIView *view = scene.keyWindow.rootViewController.view;
      view.isAccessibilityElement = YES;
      view.accessibilityIdentifier = @"scoop-surface";
      view.accessibilityValue = stage == 2 ? @"presented" : @"resumed";
    } else {
      dispatch_async(dispatch_get_main_queue(), ^{
        assert(api.scene(api.context, (__bridge void *)scene, 2));
      });
    }
  } else if (stage == 3) {
    assert(!contentClosed);
    contentClosed = YES;
  } else if (stage == 4) {
    assert(contentClosed);
    dispatch_async(dispatch_get_main_queue(), ^{
      assert(api.scene(api.context, (__bridge void *)scene, 3));
      puts("scoop-ios-surface-ok");
      fflush(stdout);
#if TARGET_OS_SIMULATOR
      exit(0);
#endif
    });
  } else {
    abort();
  }
}

@interface SurfaceAppDelegate : UIResponder <UIApplicationDelegate>
@end
@implementation SurfaceAppDelegate
- (BOOL)application:(UIApplication *)application
    didFinishLaunchingWithOptions:(NSDictionary *)options {
  observe = [NSProcessInfo.processInfo.arguments containsObject:@"--observe"];
  ScoopSurfaceHost host = {report};
  ScoopAppError error = {0};
  assert(scoop_app_init(&host, &api, &error) == SCOOP_APP_INIT_OK);
  return YES;
}
@end

@interface SurfaceProbeScene : UIResponder <UIWindowSceneDelegate>
@end
@implementation SurfaceProbeScene
- (void)scene:(UIScene *)connected
    willConnectToSession:(UISceneSession *)session
                 options:(UISceneConnectionOptions *)options {
  scene = (UIWindowScene *)connected;
  assert(api.scene(api.context, (__bridge void *)connected, 1));
}
- (void)sceneDidDisconnect:(UIScene *)disconnected {
  assert(api.scene(api.context, (__bridge void *)disconnected, 2));
}
@end

int main(int argc, char **argv) {
  @autoreleasepool {
    return UIApplicationMain(argc, argv, nil,
                             NSStringFromClass(SurfaceAppDelegate.class));
  }
}
