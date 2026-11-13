#include "loop_test.h"
#include <process.h>

typedef struct {
    ScoopIoLoop *loop;
    HANDLE proceed;
} Wake;

static unsigned __stdcall wake(void *context) {
    Wake *value = context;
    if (WaitForSingleObject(value->proceed, 5000) != WAIT_OBJECT_0)
        return 1;
    scoop_io_loop_wake(value->loop);
    return 0;
}

int scoop_test_iocp_notification(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    HANDLE notification = scoop_io_loop_event(loop);
    DWORD flags;
    CHECK(GetHandleInformation(notification, &flags) && !(flags & HANDLE_FLAG_INHERIT));
    CHECK(WaitForSingleObject(notification, 0) == WAIT_TIMEOUT);
    for (unsigned i = 0; i < 1000; ++i)
        scoop_io_loop_wake(loop);
    CHECK(WaitForSingleObject(notification, 0) == WAIT_OBJECT_0);
    CHECK(WaitForSingleObject(notification, 0) == WAIT_OBJECT_0);
    CHECK(!scoop_io_loop_poll(loop, false).code);
    CHECK(WaitForSingleObject(notification, 25) == WAIT_TIMEOUT);
    Wake value = {loop, CreateEventW(NULL, TRUE, FALSE, NULL)};
    CHECK(value.proceed);
    HANDLE thread = (HANDLE)_beginthreadex(NULL, 0, wake, &value, 0, NULL);
    CHECK(thread);
    CHECK(SetEvent(value.proceed));
    CHECK(!test_wait(loop));
    CHECK(WaitForSingleObject(thread, 5000) == WAIT_OBJECT_0);
    DWORD result;
    CHECK(GetExitCodeThread(thread, &result) && result == 0);
    CHECK(CloseHandle(thread) && CloseHandle(value.proceed));
    CHECK(!scoop_io_loop_poll(loop, false).code);
    CHECK(WaitForSingleObject(notification, 25) == WAIT_TIMEOUT);
    CHECK(!test_drain(loop));
    CHECK(!GetHandleInformation(notification, &flags) && GetLastError() == ERROR_INVALID_HANDLE);
    return 0;
}

static int publish_completions(ScoopIoLoop *loop, int64_t first, unsigned count) {
    for (unsigned i = 0; i < count; ++i) {
        ScoopIoCompletion *completion = calloc(1, sizeof(*completion));
        CHECK(completion);
        completion->event.id = first + i;
        completion->event.kind = SCOOP_IO_DONE;
        scoop_io_complete(loop, completion, 0);
    }
    return 0;
}

enum { PUBLICATION_ROUNDS = 32, PUBLICATION_COUNT = SCOOP_IO_BATCH_CAPACITY + 1 };

static unsigned __stdcall publish(void *context) {
    Wake *value = context;
    for (unsigned round = 0; round < PUBLICATION_ROUNDS; ++round) {
        CHECK(WaitForSingleObject(value->proceed, 5000) == WAIT_OBJECT_0);
        CHECK(
            !publish_completions(value->loop, 1000 + round * PUBLICATION_COUNT, PUBLICATION_COUNT));
    }
    return 0;
}

int scoop_test_iocp_notification_batches(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    HANDLE notification = scoop_io_loop_event(loop);
    unsigned total = SCOOP_IO_BATCH_CAPACITY * 2 + 3;
    CHECK(!publish_completions(loop, 1, total));
    unsigned received = 0;
    while (received < total) {
        CHECK(WaitForSingleObject(notification, 0) == WAIT_OBJECT_0);
        CHECK(!scoop_io_loop_poll(loop, false).code);
        ScoopIoBatch batch = scoop_io_loop_take_batch(loop);
        unsigned remaining = total - received;
        CHECK(batch.count ==
              (remaining > SCOOP_IO_BATCH_CAPACITY ? SCOOP_IO_BATCH_CAPACITY : remaining));
        for (int64_t i = 0; i < batch.count; ++i) {
            CHECK(batch.records[i].id == ++received && batch.records[i].kind == SCOOP_IO_DONE);
        }
        scoop_io_loop_rearm(loop, false);
    }
    CHECK(WaitForSingleObject(notification, 25) == WAIT_TIMEOUT);
    CHECK(!test_completion_count(loop));

    Wake value = {loop, CreateEventW(NULL, FALSE, FALSE, NULL)};
    CHECK(value.proceed);
    HANDLE thread = (HANDLE)_beginthreadex(NULL, 0, publish, &value, 0, NULL);
    CHECK(thread);
    for (unsigned round = 0; round < PUBLICATION_ROUNDS; ++round) {
        CHECK(SetEvent(value.proceed));
        received = 0;
        ULONGLONG deadline = GetTickCount64() + 5000;
        while (received < PUBLICATION_COUNT) {
            /* Race queue publication with consuming the previous hint and taking a bounded batch.
             */
            CHECK(!scoop_io_loop_poll(loop, false).code);
            ScoopIoBatch batch = scoop_io_loop_take_batch(loop);
            for (int64_t i = 0; i < batch.count; ++i) {
                CHECK(batch.records[i].id == 1000 + round * PUBLICATION_COUNT + received);
                CHECK(batch.records[i].kind == SCOOP_IO_DONE);
                received++;
            }
            CHECK(received <= PUBLICATION_COUNT);
            scoop_io_loop_rearm(loop, false);
            if (received < PUBLICATION_COUNT)
                CHECK(GetTickCount64() < deadline && !test_wait(loop));
        }
    }
    CHECK(WaitForSingleObject(thread, 5000) == WAIT_OBJECT_0);
    DWORD result;
    CHECK(GetExitCodeThread(thread, &result) && result == 0);
    CHECK(CloseHandle(thread) && CloseHandle(value.proceed));
    CHECK(!scoop_io_loop_poll(loop, false).code);
    CHECK(!test_completion_count(loop));
    CHECK(WaitForSingleObject(notification, 25) == WAIT_TIMEOUT);
    CHECK(!test_drain(loop));
    return 0;
}

int scoop_test_iocp_timers(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    CHECK(!scoop_io_timer_start(loop, 1, INT64_MAX, 999999999).code);
    CHECK(!scoop_io_timer_start(loop, 2, 5, 0).code);
    LARGE_INTEGER start, finish, frequency;
    CHECK(QueryPerformanceFrequency(&frequency) && QueryPerformanceCounter(&start));
    CHECK(!scoop_io_timer_start(loop, 3, 0, 25000000).code);
    ScoopIoEvent event;
    CHECK(!test_result(loop, &event));
    CHECK(event.id == 3 && event.kind == SCOOP_IO_DONE);
    CHECK(QueryPerformanceCounter(&finish));
    CHECK((long double)(finish.QuadPart - start.QuadPart) * 1000 >= frequency.QuadPart * 25.0L);
    scoop_io_timer_cancel(loop, 1);
    scoop_io_timer_cancel(loop, 1);
    scoop_io_timer_cancel(loop, 2);
    CHECK(!test_result(loop, &event) && event.id == 1);
    CHECK(!test_result(loop, &event) && event.id == 2);
    CHECK(!test_completion_count(loop));
    for (int64_t id = 10; id < 522; ++id)
        CHECK(!scoop_io_timer_start(loop, id, 10000, 0).code);
    for (int64_t id = 521; id >= 10; --id)
        scoop_io_timer_cancel(loop, id);
    for (int64_t id = 521; id >= 10; --id)
        CHECK(!test_result(loop, &event) && event.id == id);
    CHECK(!test_completion_count(loop));
    CHECK(!test_drain(loop));
    return 0;
}

ScoopIoStatus scoop_test_iocp_error(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    if (created.status.code)
        return created.status;
    ScoopIoLoop *loop = created.pointer;
    AcquireSRWLockExclusive(&loop->mutex);
    scoop_io_fail_locked(loop, scoop_io_win32(UINT32_C(0xe1234567)));
    scoop_io_notify_locked(loop);
    ReleaseSRWLockExclusive(&loop->mutex);
    ScoopIoStatus result = scoop_io_loop_poll(loop, false);
    if (test_drain(loop))
        return scoop_io_errno(-EIO);
    return result;
}

int scoop_test_iocp_drain(ScoopIoLoop *loop) { return test_drain(loop); }
