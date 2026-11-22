@import Foundation;
@import ScoopProbe;

#include <assert.h>
#include <dispatch/dispatch.h>
#include <stdio.h>

static int32_t service(int32_t value) {
    return value * 5;
}

int main(void) {
    @autoreleasepool {
        ScoopTestHostApi host = {.service = service};
        ScoopTestAppApi app = {0};
        ScoopAppError error = {0};
        assert(scoop_app_init(&host, &app, &error) == SCOOP_APP_INIT_OK);
        assert(app.run(11, app.context) == 90);
        dispatch_semaphore_t finished = dispatch_semaphore_create(0);
        dispatch_async(dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
            assert(!NSThread.isMainThread);
            assert(app.run(13, app.context) == 100);
            dispatch_semaphore_signal(finished);
        });
        assert(dispatch_semaphore_wait(finished, dispatch_time(DISPATCH_TIME_NOW, 10 * NSEC_PER_SEC)) == 0);
        app.close(app.close_context);
        puts("scoop-objc-moving-gc-ok");
    }
    return 0;
}
