#if defined(_WIN32)
#include "../windows/backend.h"
#define work_lock(loop) AcquireSRWLockExclusive(&(loop)->mutex)
#define work_unlock(loop) ReleaseSRWLockExclusive(&(loop)->mutex)
#elif defined(__APPLE__)
#include "../darwin/backend.h"
#define work_lock(loop) pthread_mutex_lock(&(loop)->mutex)
#define work_unlock(loop) pthread_mutex_unlock(&(loop)->mutex)
#else
#include "../linux/backend.h"
#define work_lock(loop) pthread_mutex_lock(&(loop)->mutex)
#define work_unlock(loop) pthread_mutex_unlock(&(loop)->mutex)
#endif
#include <errno.h>

// A queued pool entry outlives its payload when cancellation wins before execution.
struct ScoopIoWorkTask {
    ScoopIoLoop *loop;
    ScoopIoWork *work;
    bool dispatched;
};

static void append_finished(ScoopIoLoop *loop, ScoopIoWork *work) {
    work->state = SCOOP_WORK_FINISHED;
    work->queued = NULL;
    if (loop->finished_tail)
        loop->finished_tail->queued = work;
    else
        loop->finished_head = work;
    loop->finished_tail = work;
    scoop_io_notify_locked(loop);
}

static void remove_pending(ScoopIoLoop *loop, ScoopIoWork *work) {
    ScoopIoWork *previous = NULL;
    ScoopIoWork **entry = &loop->pending_head;
    while (*entry != work) {
        previous = *entry;
        entry = &(*entry)->queued;
    }
    *entry = work->queued;
    if (loop->pending_tail == work)
        loop->pending_tail = previous;
    work->queued = NULL;
}

int scoop_io_work_submit(ScoopIoWork *work, ScoopIoLoop *loop, int64_t id,
                         ScoopIoWorkExecute execute, ScoopIoWorkFinish finish) {
    ScoopIoWorkTask *task = calloc(1, sizeof(*task));
    if (!task)
        return -ENOMEM;
    task->loop = loop;
    task->work = work;
    work->loop = loop;
    work->completion.event.id = id;
    work->execute = execute;
    work->finish = finish;
    work->state = SCOOP_WORK_QUEUED;
    work->task = task;
    work_lock(loop);
    work->next = loop->work;
    loop->work = work;
    work->queued = NULL;
    if (loop->pending_tail)
        loop->pending_tail->queued = work;
    else
        loop->pending_head = work;
    loop->pending_tail = work;
    scoop_io_notify_locked(loop);
    work_unlock(loop);
    return 0;
}

ScoopIoWorkTask *scoop_io_work_take(ScoopIoLoop *loop) {
    work_lock(loop);
    ScoopIoWork *work = loop->pending_head;
    ScoopIoWorkTask *task = NULL;
    if (work) {
        remove_pending(loop, work);
        task = work->task;
        task->dispatched = true;
        loop->work_runners++;
    }
    work_unlock(loop);
    return task;
}

static void run_task(ScoopIoWorkTask *task, bool execute) {
    ScoopIoLoop *loop = task->loop;
    work_lock(loop);
    ScoopIoWork *work = task->work;
    if (work) {
        work->task = NULL;
        work->state = SCOOP_WORK_RUNNING;
    }
    work_unlock(loop);
    if (work) {
        if (execute)
            work->execute(work);
        else
            work->status = -ECANCELED;
    }
    work_lock(loop);
    if (work)
        append_finished(loop, work);
    loop->work_runners--;
    free(task);
    scoop_io_notify_locked(loop);
    work_unlock(loop);
}

void scoop_io_work_run(ScoopIoWorkTask *task) { run_task(task, true); }
void scoop_io_work_reject(ScoopIoWorkTask *task) { run_task(task, false); }

void scoop_io_work_cancel(ScoopIoLoop *loop, int64_t id) {
#if !defined(__APPLE__)
    scoop_io_request_cancel(loop, id);
#endif
    work_lock(loop);
    ScoopIoWork *work = loop->work;
    while (work && work->completion.event.id != id)
        work = work->next;
    if (work && work->state == SCOOP_WORK_QUEUED) {
        ScoopIoWorkTask *task = work->task;
        work->task = NULL;
        task->work = NULL;
        if (!task->dispatched) {
            remove_pending(loop, work);
            free(task);
        }
        work->status = -ECANCELED;
        append_finished(loop, work);
    }
    work_unlock(loop);
}

void scoop_io_work_poll(ScoopIoLoop *loop) {
    work_lock(loop);
    ScoopIoWork *work = loop->finished_head;
    loop->finished_head = loop->finished_tail = NULL;
    work_unlock(loop);
    while (work) {
        ScoopIoWork *next = work->queued;
        work->finish(work, work->status);
        work = next;
    }
}

void scoop_io_work_retire(ScoopIoWork *work) {
    ScoopIoLoop *loop = work->loop;
    work_lock(loop);
    ScoopIoWork **entry = &loop->work;
    while (*entry != work)
        entry = &(*entry)->next;
    *entry = work->next;
    work_unlock(loop);
}

void scoop_io_work_finished(ScoopIoWork *work, int status) {
    scoop_io_work_retire(work);
    scoop_io_complete(work->loop, &work->completion, status);
}
