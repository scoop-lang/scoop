#import <Foundation/Foundation.h>
#include "scoop_rt.h"

int64_t scoop_test_last_moved(void) {
    @autoreleasepool {
        NSNumber *moved = @(scoop_rt_gc_debug_last_moved_count());
        return moved.longLongValue;
    }
}
