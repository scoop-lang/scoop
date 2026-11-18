#import "window.h"

int64_t scoop_window_haptic_feedback(void *pointer, int32_t pattern) {
  @autoreleasepool {
    @try {
      SWWindow *window = (__bridge SWWindow *)pointer;
      if (window.destroying || window.inputBlocked || !window.window.visible)
        return 1;
      NSHapticFeedbackPattern value = NSHapticFeedbackPatternGeneric;
      if (pattern == 1)
        value = NSHapticFeedbackPatternAlignment;
      else if (pattern == 2)
        value = NSHapticFeedbackPatternLevelChange;
      [NSHapticFeedbackManager.defaultPerformer
          performFeedbackPattern:value
                 performanceTime:NSHapticFeedbackPerformanceTimeNow];
      return 0;
    } @catch (NSException *exception) {
      return 5;
    }
  }
}
