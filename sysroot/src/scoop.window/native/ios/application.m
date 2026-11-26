#import "window.h"
#include <math.h>

@implementation SWIEvent
@end

static void wake(SWIApplication *application) {
  CFRunLoopSourceSignal(application.source);
  CFRunLoopWakeUp(CFRunLoopGetMain());
}

@implementation SWIApplication
- (void)enqueue:(SWIEvent *)event {
  if (self.callback) {
    if (self.lastEvent)
      self.lastEvent.next = event;
    else
      self.firstEvent = event;
    self.lastEvent = event;
    wake(self);
  }
}
- (void)memoryWarning:(NSNotification *)notification {
  SWIEvent *event = [SWIEvent new];
  event.header = (SWEventHeader){.kind = 103};
  [self enqueue:event];
}
@end

static void deliver(void *info) {
  SWIApplication *app = (__bridge SWIApplication *)info;
  if (app.callback && !app.callback(app.context, 0))
    [NSException raise:NSInternalInconsistencyException
                format:@"Scoop window event handler failed"];
}
static void deadline(CFRunLoopTimerRef timer, void *info) {
  SWIApplication *app = (__bridge SWIApplication *)info;
  if (app.callback && !app.callback(app.context, 1))
    [NSException raise:NSInternalInconsistencyException
                format:@"Scoop window deadline handler failed"];
}

int32_t scoop_window_is_main_thread(void) { return NSThread.isMainThread; }

void *scoop_window_ios_application_create(SWICallback callback, void *context) {
  SWIApplication *app = [SWIApplication new];
  app.callback = callback;
  app.context = context;
  CFRunLoopSourceContext sourceContext = {0};
  sourceContext.info = (__bridge void *)app;
  sourceContext.perform = deliver;
  app.source = CFRunLoopSourceCreate(NULL, 0, &sourceContext);
  if (!app.source)
    return NULL;
  CFRunLoopAddSource(CFRunLoopGetMain(), app.source, kCFRunLoopCommonModes);
  [NSNotificationCenter.defaultCenter
      addObserver:app
         selector:@selector(memoryWarning:)
             name:UIApplicationDidReceiveMemoryWarningNotification
           object:nil];
  return (__bridge_retained void *)app;
}

void scoop_window_application_wake(void *pointer) {
  wake((__bridge SWIApplication *)pointer);
}
void scoop_window_application_deadline(void *pointer, double seconds) {
  SWIApplication *app = (__bridge SWIApplication *)pointer;
  if (app.timer) {
    CFRunLoopTimerInvalidate(app.timer);
    CFRelease(app.timer);
    app.timer = NULL;
  }
  if (seconds >= 0 && isfinite(seconds) && app.callback) {
    CFRunLoopTimerContext context = {0, pointer, NULL, NULL, NULL};
    app.timer = CFRunLoopTimerCreate(NULL, CFAbsoluteTimeGetCurrent() + seconds,
                                     0, 0, 0, deadline, &context);
    if (app.timer)
      CFRunLoopAddTimer(CFRunLoopGetMain(), app.timer, kCFRunLoopCommonModes);
  }
}
void scoop_window_ios_application_destroy(void *pointer) {
  SWIApplication *app = (__bridge_transfer SWIApplication *)pointer;
  [NSNotificationCenter.defaultCenter removeObserver:app];
  app.callback = NULL;
  scoop_window_application_deadline((__bridge void *)app, -1);
  CFRunLoopSourceInvalidate(app.source);
  CFRelease(app.source);
  app.source = NULL;
  app.firstEvent = nil;
  app.lastEvent = nil;
}
void *scoop_window_next_event(void *pointer) {
  SWIApplication *app = (__bridge SWIApplication *)pointer;
  SWIEvent *event = app.firstEvent;
  if (!event)
    return NULL;
  app.firstEvent = event.next;
  event.next = nil;
  if (!app.firstEvent)
    app.lastEvent = nil;
  return (__bridge_retained void *)event;
}
SWEventHeader scoop_window_event_header(void *pointer) {
  return ((__bridge SWIEvent *)pointer).header;
}
SWEventData scoop_window_event_data(void *pointer) {
  return ((__bridge SWIEvent *)pointer).data;
}
SWISnapshot scoop_window_ios_event_snapshot(void *pointer) {
  return ((__bridge SWIEvent *)pointer).snapshot;
}
void scoop_window_event_release(void *pointer) {
  SWIEvent *event = (__bridge_transfer SWIEvent *)pointer;
  (void)event;
}

SWInteractionPreferences scoop_window_interaction_preferences(void *pointer) {
  SWIApplication *app = (__bridge SWIApplication *)pointer;
  SWInteractionPreferences preferences = sw_unknown_preferences();
  UITraitCollection *traits = app.preferenceView.traitCollection;
  preferences.reduce_motion = UIAccessibilityIsReduceMotionEnabled();
  preferences.reduce_transparency =
      UIAccessibilityIsReduceTransparencyEnabled();
  if (traits) {
    preferences.high_contrast =
        traits.accessibilityContrast == UIAccessibilityContrastHigh;
    preferences.text_scale_factor =
        [UIFontMetrics.defaultMetrics scaledValueForValue:100
                            compatibleWithTraitCollection:traits] /
        100.0;
  }
  return preferences;
}
