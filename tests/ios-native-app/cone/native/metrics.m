#import <Foundation/Foundation.h>
#include "scoop_rt.h"

int64_t scoop_test_last_moved(void) {
    @autoreleasepool {
        BOOL recent;
        if (@available(iOS 26.0, *))
            recent = YES;
        else
            recent = NO;
        if (recent != [NSProcessInfo.processInfo isOperatingSystemAtLeastVersion:
                          (NSOperatingSystemVersion){26, 0, 0}])
            return -1;
        NSNumber *moved = @(scoop_rt_gc_debug_last_moved_count());
        return moved.longLongValue;
    }
}
