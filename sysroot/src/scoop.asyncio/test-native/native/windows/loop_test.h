#ifndef SCOOP_ASYNCIO_WINDOWS_LOOP_TEST_H
#define SCOOP_ASYNCIO_WINDOWS_LOOP_TEST_H

#include "../../../native/windows/backend.h"
#include "../completion_queue.h"
#include <errno.h>

#define CHECK(condition)                                                                           \
    do {                                                                                           \
        if (!(condition))                                                                          \
            return __LINE__;                                                                       \
    } while (0)

static inline int test_wait(ScoopIoLoop *loop) {
    return WaitForSingleObject(scoop_io_loop_event(loop), 5000) == WAIT_OBJECT_0 ? 0 : -1;
}

// Pure native tests drive synchronous helpers explicitly; managed pool behavior has Cone tests.
static inline void test_work(ScoopIoLoop *loop) {
    ScoopIoWorkTask *task;
    while ((task = scoop_io_work_take(loop)))
        scoop_io_work_run(task);
}

static inline int test_drain(ScoopIoLoop *loop) {
    ULONGLONG deadline = GetTickCount64() + 5000;
    for (;;) {
        test_work(loop);
        if (scoop_io_loop_poll(loop, false).code)
            return -1;
        while (test_completion_count(loop))
            scoop_io_loop_take_batch(loop);
        scoop_io_loop_rearm(loop, false);
        if (scoop_io_loop_idle(loop))
            return scoop_io_loop_destroy(loop).code;
        if (GetTickCount64() >= deadline || test_wait(loop))
            return -1;
    }
}

static inline int test_result(ScoopIoLoop *loop, ScoopIoEvent *event) {
    ULONGLONG deadline = GetTickCount64() + 5000;
    for (;;) {
        test_work(loop);
        if (scoop_io_loop_poll(loop, false).code)
            return -1;
        if (test_completion_count(loop)) {
            *event = test_completion_take(loop);
            return 0;
        }
        if (GetTickCount64() >= deadline || test_wait(loop))
            return -1;
    }
}

#endif
