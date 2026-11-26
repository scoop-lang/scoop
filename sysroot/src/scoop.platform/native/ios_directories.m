#import <Foundation/Foundation.h>
#include "environment.h"
#include "path_strings.h"
#include <errno.h>
#include <string.h>

static ScoopPlatformString copy_directory(NSString *directory) {
    const char *path = directory.fileSystemRepresentation;
    if (!path || !*path)
        return (ScoopPlatformString){NULL, 0, ENOENT};
    char *copy = strdup(path);
    if (!copy)
        return (ScoopPlatformString){NULL, 0, ENOMEM};
    return (ScoopPlatformString){copy, (int64_t)strlen(copy), 0};
}

ScoopPlatformString scoop_platform_home_directory_result(void) {
    @autoreleasepool {
        return copy_directory(NSHomeDirectory());
    }
}

ScoopPlatformString scoop_platform_temporary_directory_result(void) {
    @autoreleasepool {
        return copy_directory(NSTemporaryDirectory());
    }
}
