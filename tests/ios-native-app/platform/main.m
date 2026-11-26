@import ScoopProbe;
@import Foundation;
#include <stdio.h>

int main(void) {
    @autoreleasepool {
        NSBundle *bundle = NSBundle.mainBundle;
        NSURL *resource = [bundle URLForResource:@"resource" withExtension:@"txt"];
        if (!resource)
            return 1;
        ScoopPlatformHost host = {NSHomeDirectory().fileSystemRepresentation,
                                  NSTemporaryDirectory().fileSystemRepresentation,
                                  bundle.executableURL.fileSystemRepresentation,
                                  resource.fileSystemRepresentation};
        int result = 0;
        ScoopAppError error = {0};
        if (scoop_app_init(&host, &result, &error) != SCOOP_APP_INIT_OK) {
            if (error.bytes)
                fwrite(error.bytes, 1, error.length, stderr);
            return 1;
        }
        if (result != 1)
            return 1;
        puts("scoop-ios-platform-ok");
        return 0;
    }
}
