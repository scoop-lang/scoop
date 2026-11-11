#ifndef SCOOP_ASYNCIO_TEST_COMPLETION_QUEUE_H
#define SCOOP_ASYNCIO_TEST_COMPLETION_QUEUE_H

#if defined(_WIN32)
#include "../../native/windows/backend.h"
#define test_queue_lock(loop) AcquireSRWLockExclusive(&(loop)->mutex)
#define test_queue_unlock(loop) ReleaseSRWLockExclusive(&(loop)->mutex)
#elif defined(__APPLE__)
#include "../../native/darwin/backend.h"
#define test_queue_lock(loop) pthread_mutex_lock(&(loop)->mutex)
#define test_queue_unlock(loop) pthread_mutex_unlock(&(loop)->mutex)
#else
#include "../../native/linux/backend.h"
#define test_queue_lock(loop) ((void)0)
#define test_queue_unlock(loop) ((void)0)
#endif

// Native retirement tests observe queued completions before managed delivery.
static inline int64_t test_completion_count(ScoopIoLoop *loop) {
    test_queue_lock(loop);
    int64_t count = loop->completions;
    test_queue_unlock(loop);
    return count;
}

// Windows socket tests consume individual results while retaining the other queued events.
static inline ScoopIoEvent test_completion_take(ScoopIoLoop *loop) {
    test_queue_lock(loop);
    ScoopIoCompletion *entry = loop->head;
    if (entry) {
        loop->head = entry->next;
        if (!loop->head)
            loop->tail = NULL;
        loop->completions--;
    }
    test_queue_unlock(loop);
    if (!entry)
        return (ScoopIoEvent){0};
    ScoopIoEvent event = entry->event;
    free(entry);
    return event;
}

#endif
