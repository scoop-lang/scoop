@import ScoopProbe;
@import UIKit;

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

@interface ProtectedDataDelegate : UIResponder <UIApplicationDelegate>
- (BOOL)check:(UIApplication *)application event:(const char *)event;
@end

@interface ProtectedDataScene : UIResponder <UIWindowSceneDelegate>
@property(nonatomic, strong) UIWindow *window;
@end

@implementation ProtectedDataDelegate {
    ScoopProtectedDataApi api;
}

- (BOOL)check:(UIApplication *)application event:(const char *)event {
    bool actual = api.query(api.context);
    bool expected = application.protectedDataAvailable;
    printf("%s scoop=%d UIKit=%d\n", event, actual, expected);
    fflush(stdout);
    assert(actual == expected);
    return actual;
}

- (BOOL)application:(UIApplication *)application didFinishLaunchingWithOptions:(NSDictionary *)options {
    (void)options;
    ScoopAppError error = {0};
    int host = 0;
    assert(scoop_app_init(&host, &api, &error) == SCOOP_APP_INIT_OK);
    [self check:application event:"launch"];
    return YES;
}

- (void)applicationProtectedDataWillBecomeUnavailable:(UIApplication *)application {
    [self check:application event:"will-lock"];
}
- (void)applicationProtectedDataDidBecomeAvailable:(UIApplication *)application {
    [self check:application event:"did-unlock"];
}
@end

@implementation ProtectedDataScene {
    UIBackgroundTaskIdentifier samplingTask;
    dispatch_source_t samplingTimer;
}
- (void)scene:(UIScene *)scene
    willConnectToSession:(UISceneSession *)session
                 options:(UISceneConnectionOptions *)options {
    samplingTask = UIBackgroundTaskInvalid;
    (void)session;
    (void)options;
    self.window = [[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController = [UIViewController new];
    [self.window makeKeyAndVisible];
}

- (void)stopSampling {
    if (samplingTimer) {
        dispatch_source_cancel(samplingTimer);
        samplingTimer = nil;
    }
    if (samplingTask != UIBackgroundTaskInvalid) {
        [UIApplication.sharedApplication endBackgroundTask:samplingTask];
        samplingTask = UIBackgroundTaskInvalid;
    }
}

- (void)sceneDidEnterBackground:(UIScene *)scene {
    (void)scene;
    UIApplication *app = UIApplication.sharedApplication;
    [self stopSampling];
    samplingTask = [app beginBackgroundTaskWithName:@"Compare protected data after lock"
                                  expirationHandler:^{
                                    [self stopSampling];
                                  }];
    if (samplingTask == UIBackgroundTaskInvalid)
        return;
    // The will-lock notification can arrive before data actually becomes unavailable.
    __block unsigned samples = 0;
    samplingTimer = dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0, dispatch_get_main_queue());
    dispatch_source_set_timer(samplingTimer, dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC),
                              NSEC_PER_SEC, NSEC_PER_SEC / 10);
    dispatch_source_set_event_handler(samplingTimer, ^{
      BOOL available = [(ProtectedDataDelegate *)app.delegate check:app event:"background"];
      if (!available || ++samples == 25)
          [self stopSampling];
    });
    dispatch_resume(samplingTimer);
}

- (void)sceneDidBecomeActive:(UIScene *)scene {
    (void)scene;
    [self stopSampling];
    UIApplication *application = UIApplication.sharedApplication;
    [(ProtectedDataDelegate *)application.delegate check:application event:"active"];
#if TARGET_OS_SIMULATOR
    puts("scoop-ios-protected-data-ok");
    fflush(stdout);
    exit(0);
#endif
}

@end

int main(int argc, char **argv) {
    @autoreleasepool {
        return UIApplicationMain(argc, argv, nil, NSStringFromClass(ProtectedDataDelegate.class));
    }
}
