@import ScoopProbe;
@import Foundation;
#include <stdio.h>

int main(void) {
  @autoreleasepool {
    ScoopAsyncResult result = 0;
    ScoopAppError error = {0};
    if (scoop_app_init(&result, &result, &error) != SCOOP_APP_INIT_OK) {
      if (error.bytes)
        fwrite(error.bytes, 1, error.length, stderr);
      return 1;
    }
    if (result)
      return 1;
    NSDate *limit = [NSDate dateWithTimeIntervalSinceNow:5];
    while (!result && limit.timeIntervalSinceNow > 0)
      [NSRunLoop.mainRunLoop
          runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.02]];
    if (result != 1)
      return 1;
    puts("scoop-ios-asyncio-ok");
    return 0;
  }
}
