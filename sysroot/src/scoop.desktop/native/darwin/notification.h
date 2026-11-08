#include "../notification/common.h"
#import <AppKit/AppKit.h>
#import <UserNotifications/UserNotifications.h>

@interface SNMacPost : NSObject
@property(nonatomic) int64_t request;
@property(nonatomic, strong) UNMutableNotificationContent *content;
@property(nonatomic, strong) UNNotificationCategory *category;
@property(nonatomic, copy) NSString *identifier;
@end

@interface SNMac : NSObject <UNUserNotificationCenterDelegate>
@property(nonatomic, strong) UNUserNotificationCenter *center;
@property(nonatomic, strong) id<UNUserNotificationCenterDelegate> previous;
@property(nonatomic) CFRunLoopSourceRef source;
@property(nonatomic) SDCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) SNEvent *head;
@property(nonatomic) SNEvent *tail;
@property(nonatomic) BOOL closed;
@property(nonatomic) BOOL cold;
@property(nonatomic) BOOL categoriesLoaded;
@property(nonatomic) BOOL loadingCategories;
@property(nonatomic, strong)
    NSMutableDictionary<NSString *, UNNotificationCategory *> *categories;
@property(nonatomic, strong) NSMutableArray<SNMacPost *> *pending;
- (void)enqueue:(SNEvent *)event;
- (void)settings:(int64_t)request kind:(int32_t)kind;
- (void)submit:(SNMacPost *)post;
@end

int sn_mac_availability(UNNotificationSettings *settings);
void sn_mac_error(SNEvent *event, NSError *error);
