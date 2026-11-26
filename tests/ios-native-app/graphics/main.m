@import ScoopProbe;
@import Foundation;
#include <stdio.h>

int main(void) {
    @autoreleasepool {
        int host = 0;
        ScoopGraphicsResult result = {0};
        ScoopAppError error = {0};
        if (scoop_app_init(&host, &result, &error) != SCOOP_APP_INIT_OK) {
            if (error.bytes)
                fwrite(error.bytes, 1, error.length, stderr);
            return 1;
        }
        printf("text lines=%d, covered pixels=%d\n", result.lines, result.textPixels);
        if (result.lines < 2 || result.textPixels < 1)
            return 1;
        puts("scoop-ios-graphics-ok");
        return 0;
    }
}
