@import ScoopProbe;
@import UIKit;
#include <assert.h>
#include <stdio.h>

static ScoopUiApi api;
@interface UiAppDelegate : UIResponder <UIApplicationDelegate>
@end
@implementation UiAppDelegate
- (BOOL)application:(UIApplication *)application
    didFinishLaunchingWithOptions:(NSDictionary *)options {
  ScoopAppError error = {0};
  int32_t status = scoop_app_init((__bridge void *)application, &api, &error);
  if (status != SCOOP_APP_INIT_OK)
    fprintf(stderr, "Scoop initialization failed: %.*s\n", (int)error.length,
            error.bytes);
  return status == SCOOP_APP_INIT_OK;
}
- (void)applicationWillTerminate:(UIApplication *)application {
  assert(api.scene(api.context, (__bridge void *)application, 3));
}
@end
@interface UiSceneDelegate : UIResponder <UIWindowSceneDelegate>
@end
@implementation UiSceneDelegate
- (void)scene:(UIScene *)scene
    willConnectToSession:(UISceneSession *)session
                 options:(UISceneConnectionOptions *)options {
  assert(api.scene(api.context, (__bridge void *)scene, 1));
}
- (void)sceneDidDisconnect:(UIScene *)scene {
  assert(api.scene(api.context, (__bridge void *)scene, 2));
}
@end
int main(int argc, char **argv) {
  @autoreleasepool {
    return UIApplicationMain(argc, argv, nil,
                             NSStringFromClass(UiAppDelegate.class));
  }
}
