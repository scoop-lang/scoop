#include "../preferences.h"
#import "window.h"

SWInteractionPreferences scoop_window_interaction_preferences(void *pointer) {
  @autoreleasepool {
    SWInteractionPreferences value = sw_unknown_preferences();
    NSWorkspace *workspace = NSWorkspace.sharedWorkspace;
    value.auto_hide_scrollbars =
        NSScroller.preferredScrollerStyle == NSScrollerStyleOverlay;
    value.reduce_motion = workspace.accessibilityDisplayShouldReduceMotion;
    value.high_contrast = workspace.accessibilityDisplayShouldIncreaseContrast;
    value.reduce_transparency =
        workspace.accessibilityDisplayShouldReduceTransparency;
    value.double_click_seconds = NSEvent.doubleClickInterval;
    return value;
  }
}

static void observe(SWApplication *application, NSNotificationCenter *center,
                    NSNotificationName name) {
  __weak SWApplication *weak = application;
  id token = [center addObserverForName:name
                                 object:nil
                                  queue:NSOperationQueue.mainQueue
                             usingBlock:^(NSNotification *notification) {
                               SWApplication *owner = weak;
                               if (!owner.preferenceObservers)
                                 return;
                               SWEvent *event = [SWEvent new];
                               event.header = (SWEventHeader){.kind = 102};
                               SWEventData data = {0};
                               event.data = data;
                               [owner enqueue:event];
                             }];
  [application.preferenceObservers addObject:token];
}

void sw_preferences_start(SWApplication *application) {
  application.preferenceObservers = [NSMutableArray new];
  observe(application, NSNotificationCenter.defaultCenter,
          NSPreferredScrollerStyleDidChangeNotification);
  observe(application, NSWorkspace.sharedWorkspace.notificationCenter,
          NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification);
  observe(application, NSNotificationCenter.defaultCenter,
          NSUserDefaultsDidChangeNotification);
  observe(application, NSNotificationCenter.defaultCenter,
          NSApplicationDidBecomeActiveNotification);
}

void sw_preferences_stop(SWApplication *application) {
  NSArray *observers = application.preferenceObservers;
  application.preferenceObservers = nil;
  for (id token in observers) {
    [NSNotificationCenter.defaultCenter removeObserver:token];
    [NSWorkspace.sharedWorkspace.notificationCenter removeObserver:token];
  }
}
