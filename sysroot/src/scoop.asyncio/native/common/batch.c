#if defined(_WIN32)
#include "../windows/backend.h"
#define batch_lock(loop) AcquireSRWLockExclusive(&(loop)->mutex)
#define batch_unlock(loop) ReleaseSRWLockExclusive(&(loop)->mutex)
#elif defined(__APPLE__)
#include "../darwin/backend.h"
#define batch_lock(loop) pthread_mutex_lock(&(loop)->mutex)
#define batch_unlock(loop) pthread_mutex_unlock(&(loop)->mutex)
#else
#include "../linux/backend.h"
// Linux request retirement and logical completions are owned by the reactor thread.
#define batch_lock(loop) ((void)0)
#define batch_unlock(loop) ((void)0)
#endif

static void retire_batch(ScoopIoLoop *loop) {
    ScoopIoCompletion *entry = loop->retired_batch;
    loop->retired_batch = NULL;
    while (entry) {
        ScoopIoCompletion *next = entry->next;
        free(entry);
        entry = next;
    }
}

ScoopIoBatch scoop_io_loop_take_batch(ScoopIoLoop *loop) {
    retire_batch(loop);
    batch_lock(loop);
    ScoopIoCompletion *head = loop->head, *remaining = head, *last = NULL;
    int64_t count = 0;
    while (remaining && count < SCOOP_IO_BATCH_CAPACITY) {
        loop->batch[count++] = remaining->event;
        last = remaining;
        remaining = remaining->next;
    }
    loop->head = remaining;
    if (!remaining)
        loop->tail = NULL;
    if (last)
        last->next = NULL;
    loop->completions -= count;
    batch_unlock(loop);
    // Queued commands may still reference a closing endpoint until this batch is delivered.
    loop->retired_batch = head;
    return (ScoopIoBatch){loop->batch, count};
}

void scoop_io_loop_rearm(ScoopIoLoop *loop, bool managed_ready) {
    retire_batch(loop);
#if defined(_WIN32)
    AcquireSRWLockExclusive(&loop->mutex);
    if (managed_ready || loop->head)
        scoop_io_notify_locked(loop);
    ReleaseSRWLockExclusive(&loop->mutex);
#elif defined(__APPLE__)
    pthread_mutex_lock(&loop->mutex);
    if (managed_ready || loop->head)
        scoop_io_notify_locked(loop);
    pthread_mutex_unlock(&loop->mutex);
#else
    // Coalesce owner submissions and completions at the boundary that returns to the scheduler.
    if (managed_ready || loop->head ||
        (!loop->submission_blocked &&
         (loop->pending.head || loop->control.head || io_uring_sq_ready(&loop->ring))))
        scoop_io_loop_wake(loop);
#endif
}
