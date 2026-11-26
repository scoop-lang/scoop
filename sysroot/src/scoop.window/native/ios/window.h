#ifndef SCOOP_WINDOW_IOS_H
#define SCOOP_WINDOW_IOS_H

#include "../events.h"
#include "../preferences.h"
#import <UIKit/UIKit.h>

typedef int32_t (*SWICallback)(void *, int32_t);

typedef struct {
  double width, height, scale, top, left, bottom, right;
  int32_t visible, focused, occluded, dark;
} SWISnapshot;

@interface SWIEvent : NSObject
@property(nonatomic) SWEventHeader header;
@property(nonatomic) SWEventData data;
@property(nonatomic) SWContactData contact;
@property(nonatomic) SWISnapshot snapshot;
@property(nonatomic, strong) SWIEvent *next;
@end

@interface SWIApplication : NSObject
@property(nonatomic) SWICallback callback;
@property(nonatomic) void *context;
@property(nonatomic) CFRunLoopSourceRef source;
@property(nonatomic) CFRunLoopTimerRef timer;
@property(nonatomic, strong) SWIEvent *firstEvent;
@property(nonatomic, strong) SWIEvent *lastEvent;
@property(nonatomic, weak) UIView *preferenceView;
- (void)enqueue:(SWIEvent *)event;
@end

@class SWIScene;
@interface SWIView : UIView
@property(nonatomic, weak) SWIScene *owner;
- (void)cancelContacts:(SWPointerCancelReason)reason;
@end

@interface SWIScene : NSObject
@property(nonatomic, weak) SWIApplication *application;
@property(nonatomic, strong) UIWindow *window;
@property(nonatomic, strong) SWIView *view;
@property(nonatomic) int64_t identity;
@property(nonatomic) BOOL active;
@property(nonatomic) BOOL foreground;
- (SWISnapshot)snapshot;
- (void)geometryChanged;
- (void)emit:(int32_t)kind flags:(int32_t)flags;
@end

#endif
