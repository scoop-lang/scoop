@import ScoopGallery;
@import UIKit;
#include <stdio.h>

static ScoopGalleryApi gallery;

@interface GalleryAppDelegate : UIResponder <UIApplicationDelegate>
@end
@implementation GalleryAppDelegate
- (BOOL)application:(UIApplication *)application
    didFinishLaunchingWithOptions:(NSDictionary *)options {
  ScoopAppError error = {0};
  if (scoop_app_init((__bridge void *)application, &gallery, &error) ==
      SCOOP_APP_INIT_OK)
    return YES;
  fprintf(stderr, "Scoop Gallery initialization failed: %.*s\n",
          (int)error.length, error.bytes);
  return NO;
}
@end

@interface GallerySceneDelegate : UIResponder <UIWindowSceneDelegate>
@end
@implementation GallerySceneDelegate
- (void)scene:(UIScene *)scene
    willConnectToSession:(UISceneSession *)session
                 options:(UISceneConnectionOptions *)options {
  if (!gallery.scene(gallery.context, (__bridge void *)scene,
                     ScoopGalleryConnect))
    NSLog(@"Could not connect the Scoop Gallery scene");
}
- (void)sceneDidDisconnect:(UIScene *)scene {
  if (!gallery.scene(gallery.context, (__bridge void *)scene,
                     ScoopGalleryDisconnect))
    NSLog(@"Could not disconnect the Scoop Gallery scene");
}
@end

int main(int argc, char **argv) {
  @autoreleasepool {
    return UIApplicationMain(argc, argv, nil,
                             NSStringFromClass(GalleryAppDelegate.class));
  }
}
