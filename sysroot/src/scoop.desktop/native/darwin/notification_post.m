#import "notification.h"

int64_t scoop_notification_post(void *pointer, int64_t request,
                                SNContent *content) {
  @autoreleasepool {
    SNMac *owner = (__bridge SNMac *)pointer;
    if (!owner.center) {
      SNEvent *event = sn_event(SN_POST, request);
      sn_event_error(event, SN_NOT_CONFIGURED, 0, NULL);
      [owner enqueue:event];
      return 0;
    }
    SNMacPost *post = [SNMacPost new];
    post.request = request;
    NSString *identifier =
        [NSString stringWithUTF8String:sn_content_text(content, 0)];
    post.identifier = [@"scoop:" stringByAppendingString:identifier];
    post.content = [UNMutableNotificationContent new];
    post.content.title =
        [NSString stringWithUTF8String:sn_content_text(content, 1)];
    post.content.body =
        [NSString stringWithUTF8String:sn_content_text(content, 2)];
    post.content.userInfo = @{
      @"scoop.id" : identifier,
      @"scoop.payload" :
          [NSString stringWithUTF8String:sn_content_text(content, 3)]
    };
    if (!content->silent)
      post.content.sound = UNNotificationSound.defaultSound;
    NSMutableArray<UNNotificationAction *> *actions = [NSMutableArray new];
    for (size_t index = 0; index < content->count; ++index) {
      const SNAction *action = &content->actions[index];
      [actions
          addObject:
              [UNNotificationAction
                  actionWithIdentifier:[@"scoop:"
                                           stringByAppendingString:
                                               [NSString
                                                   stringWithUTF8String:action->
                                                                        id]]
                                 title:[NSString
                                           stringWithUTF8String:action->title]
                               options:UNNotificationActionOptionForeground]];
    }
    post.category = [UNNotificationCategory
        categoryWithIdentifier:post.identifier
                       actions:actions
             intentIdentifiers:@[]
                       options:UNNotificationCategoryOptionCustomDismissAction];
    post.content.categoryIdentifier = post.identifier;
    [owner.center getNotificationSettingsWithCompletionHandler:^(
                      UNNotificationSettings *settings) {
      dispatch_async(dispatch_get_main_queue(), ^{
        if (owner.closed)
          return;
        int availability = sn_mac_availability(settings);
        if (availability == SN_AVAILABLE)
          [owner submit:post];
        else {
          SNEvent *event = sn_event(SN_POST, request);
          sn_event_error(event,
                         availability == SN_NEEDS_PERMISSION
                             ? SN_PERMISSION_REQUIRED
                             : SN_DISABLED,
                         0, NULL);
          [owner enqueue:event];
        }
      });
    }];
    return 0;
  }
}
