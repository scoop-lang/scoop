#import "notification.h"
#include <errno.h>

@implementation SNMacPost
@end

int sn_mac_availability(UNNotificationSettings *settings) {
  switch (settings.authorizationStatus) {
  case UNAuthorizationStatusNotDetermined:
    return SN_NEEDS_PERMISSION;
  case UNAuthorizationStatusDenied:
    return SN_SETTING_DISABLED;
  default:
    return SN_AVAILABLE;
  }
}
void sn_mac_error(SNEvent *event, NSError *error) {
  sn_event_error(event, SN_NATIVE_FAILURE, error.code,
                 error.localizedDescription.UTF8String);
}

static void perform(void *pointer) {
  SNMac *owner = (__bridge SNMac *)pointer;
  SDCallback callback;
  void *context;
  @synchronized(owner) {
    callback = owner.closed ? NULL : owner.callback;
    context = owner.context;
  }
  if (callback)
    callback(context, 0);
}

@implementation SNMac
- (void)enqueue:(SNEvent *)event {
  @synchronized(self) {
    if (self.closed) {
      scoop_notification_event_destroy(event);
      return;
    }
    if (self.tail)
      self.tail->next = event;
    else
      self.head = event;
    self.tail = event;
    CFRunLoopSourceSignal(self.source);
    CFRunLoopWakeUp(CFRunLoopGetMain());
  }
}
- (void)settings:(int64_t)request kind:(int32_t)kind {
  if (!self.center) {
    SNEvent *event = sn_event(kind, request);
    event->data.detail = SN_UNCONFIGURED;
    [self enqueue:event];
    return;
  }
  [self.center getNotificationSettingsWithCompletionHandler:^(
                   UNNotificationSettings *settings) {
    SNEvent *event = sn_event(kind, request);
    event->data.detail = sn_mac_availability(settings);
    event->data.capabilities = SN_ACTIONS | SN_REPLACE | SN_CAN_WITHDRAW |
                               SN_ACTIVATION | SN_CLOSE_EVENTS |
                               (self.cold ? SN_COLD_ACTIVATION : 0);
    [self enqueue:event];
  }];
}
- (void)submit:(SNMacPost *)post {
  if (self.closed)
    return;
  if (!self.categoriesLoaded) {
    [self.pending addObject:post];
    if (self.loadingCategories)
      return;
    self.loadingCategories = YES;
    [self.center getNotificationCategoriesWithCompletionHandler:^(
                     NSSet<UNNotificationCategory *> *categories) {
      dispatch_async(dispatch_get_main_queue(), ^{
        if (self.closed)
          return;
        for (UNNotificationCategory *category in categories)
          self.categories[category.identifier] = category;
        self.categoriesLoaded = YES;
        NSArray<SNMacPost *> *pending = self.pending.copy;
        [self.pending removeAllObjects];
        for (SNMacPost *value in pending)
          [self submit:value];
      });
    }];
    return;
  }
  self.categories[post.category.identifier] = post.category;
  [self.center
      setNotificationCategories:[NSSet setWithArray:self.categories.allValues]];
  UNNotificationRequest *request =
      [UNNotificationRequest requestWithIdentifier:post.identifier
                                           content:post.content
                                           trigger:nil];
  __weak SNMac *weakOwner = self;
  [self.center addNotificationRequest:request
                withCompletionHandler:^(NSError *error) {
                  SNMac *owner = weakOwner;
                  if (!owner)
                    return;
                  SNEvent *event = sn_event(SN_POST, post.request);
                  if (error)
                    sn_mac_error(event, error);
                  [owner enqueue:event];
                }];
}
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
       willPresentNotification:(UNNotification *)notification
         withCompletionHandler:
             (void (^)(UNNotificationPresentationOptions))completion {
  if ([notification.request.content.userInfo[@"scoop.id"]
          isKindOfClass:NSString.class]) {
    UNNotificationPresentationOptions options =
        UNNotificationPresentationOptionBanner |
        UNNotificationPresentationOptionList;
    if (notification.request.content.sound)
      options |= UNNotificationPresentationOptionSound;
    completion(options);
  } else if ([self.previous respondsToSelector:_cmd]) {
    [self.previous userNotificationCenter:center
                  willPresentNotification:notification
                    withCompletionHandler:completion];
  } else
    completion(UNNotificationPresentationOptionNone);
}
- (void)userNotificationCenter:(UNUserNotificationCenter *)center
    didReceiveNotificationResponse:(UNNotificationResponse *)response
             withCompletionHandler:(void (^)(void))completion {
  NSDictionary *info = response.notification.request.content.userInfo;
  NSString *identifier = info[@"scoop.id"], *payload = info[@"scoop.payload"];
  if (![identifier isKindOfClass:NSString.class] ||
      ![payload isKindOfClass:NSString.class]) {
    if ([self.previous respondsToSelector:_cmd])
      [self.previous userNotificationCenter:center
             didReceiveNotificationResponse:response
                      withCompletionHandler:completion];
    else
      completion();
    return;
  }
  NSString *action = response.actionIdentifier;
  SNEvent *event;
  if ([action isEqualToString:UNNotificationDismissActionIdentifier]) {
    event = sn_event(SN_CLOSED, 0);
    event->data.detail = 2;
  } else {
    event = sn_event(SN_ACTIVATED, 0);
    if (![action isEqualToString:UNNotificationDefaultActionIdentifier]) {
      if (![action hasPrefix:@"scoop:"]) {
        scoop_notification_event_destroy(event);
        completion();
        return;
      }
      sn_event_set(event, 1, [action substringFromIndex:6].UTF8String);
    }
    sn_event_set(event, 2, payload.UTF8String);
  }
  sn_event_set(event, 0, identifier.UTF8String);
  [self enqueue:event];
  completion();
}
@end

SDResult scoop_notification_create(const uint8_t *id, int64_t length) {
  @autoreleasepool {
    if (![NSThread isMainThread])
      return (SDResult){NULL, SD_INVALID};
    NSString *identifier =
        [[NSString alloc] initWithBytes:id
                                 length:(NSUInteger)length
                               encoding:NSUTF8StringEncoding];
    if (!identifier.length)
      return (SDResult){NULL, SD_INVALID};
    SNMac *owner = [SNMac new];
    owner.cold = NSApp == nil;
    owner.categories = [NSMutableDictionary new];
    owner.pending = [NSMutableArray new];
    CFRunLoopSourceContext context = {0};
    context.info = (__bridge void *)owner;
    context.perform = perform;
    owner.source = CFRunLoopSourceCreate(NULL, 0, &context);
    if (!owner.source)
      return (SDResult){NULL, ENOMEM};
    CFRunLoopAddSource(CFRunLoopGetMain(), owner.source, kCFRunLoopCommonModes);
    NSBundle *bundle = NSBundle.mainBundle;
    if ([bundle.bundlePath.pathExtension isEqualToString:@"app"] &&
        [bundle.bundleIdentifier isEqualToString:identifier]) {
      owner.center = UNUserNotificationCenter.currentNotificationCenter;
      owner.previous = owner.center.delegate;
      owner.center.delegate = owner;
    }
    return (SDResult){(__bridge_retained void *)owner, 0};
  }
}
int64_t scoop_notification_attach(void *pointer, SDCallback callback,
                                  void *context) {
  SNMac *owner = (__bridge SNMac *)pointer;
  @synchronized(owner) {
    owner.callback = callback;
    owner.context = context;
    if (owner.head) {
      CFRunLoopSourceSignal(owner.source);
      CFRunLoopWakeUp(CFRunLoopGetMain());
    }
  }
  return 0;
}
void scoop_notification_destroy(void *pointer) {
  SNMac *owner = (__bridge_transfer SNMac *)pointer;
  @synchronized(owner) {
    owner.closed = YES;
    owner.callback = NULL;
    CFRunLoopSourceInvalidate(owner.source);
    CFRelease(owner.source);
    owner.source = NULL;
    while (owner.head) {
      SNEvent *event = owner.head;
      owner.head = event->next;
      scoop_notification_event_destroy(event);
    }
    owner.tail = NULL;
  }
  if (owner.center.delegate == owner)
    owner.center.delegate = owner.previous;
  [owner.pending removeAllObjects];
}
SNEvent *scoop_notification_next_event(void *pointer) {
  SNMac *owner = (__bridge SNMac *)pointer;
  @synchronized(owner) {
    SNEvent *event = owner.head;
    if (event) {
      owner.head = event->next;
      if (!owner.head)
        owner.tail = NULL;
      event->next = NULL;
    }
    return event;
  }
}
int64_t scoop_notification_settings(void *pointer, int64_t request,
                                    int32_t permission) {
  SNMac *owner = (__bridge SNMac *)pointer;
  if (!permission || !owner.center) {
    [owner settings:request kind:permission ? SN_PERMISSION : SN_SETTINGS];
  } else {
    [owner.center
        requestAuthorizationWithOptions:UNAuthorizationOptionAlert |
                                        UNAuthorizationOptionSound
                      completionHandler:^(BOOL granted, NSError *error) {
                        if (error) {
                          SNEvent *event = sn_event(SN_PERMISSION, request);
                          sn_mac_error(event, error);
                          [owner enqueue:event];
                        } else
                          [owner settings:request kind:SN_PERMISSION];
                      }];
  }
  return 0;
}
int64_t scoop_notification_withdraw(void *pointer, int64_t request,
                                    const uint8_t *id, int64_t length) {
  SNMac *owner = (__bridge SNMac *)pointer;
  SNEvent *event = sn_event(SN_WITHDRAW, request);
  if (!owner.center)
    sn_event_error(event, SN_NOT_CONFIGURED, 0, NULL);
  else {
    NSString *identifier =
        [[NSString alloc] initWithBytes:id
                                 length:(NSUInteger)length
                               encoding:NSUTF8StringEncoding];
    NSArray *identifiers = @[ [@"scoop:" stringByAppendingString:identifier] ];
    [owner.center removePendingNotificationRequestsWithIdentifiers:identifiers];
    [owner.center removeDeliveredNotificationsWithIdentifiers:identifiers];
  }
  [owner enqueue:event];
  return 0;
}
