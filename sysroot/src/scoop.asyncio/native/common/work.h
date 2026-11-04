#ifndef SCOOP_ASYNCIO_COMMON_WORK_H
#define SCOOP_ASYNCIO_COMMON_WORK_H

#include "../loop.h"

typedef struct ScoopIoWork ScoopIoWork;
typedef struct ScoopIoWorkTask ScoopIoWorkTask;
typedef void (*ScoopIoWorkExecute)(ScoopIoWork *);
typedef void (*ScoopIoWorkFinish)(ScoopIoWork *, int);

typedef enum { SCOOP_WORK_QUEUED, SCOOP_WORK_RUNNING, SCOOP_WORK_FINISHED } ScoopIoWorkState;

struct ScoopIoWork {
    ScoopIoCompletion completion;
    ScoopIoLoop *loop;
    ScoopIoWork *next;
    ScoopIoWork *queued;
    ScoopIoWorkTask *task;
    ScoopIoWorkExecute execute;
    ScoopIoWorkFinish finish;
    ScoopIoWorkState state;
    int status;
};

int scoop_io_work_submit(ScoopIoWork *work, ScoopIoLoop *loop, int64_t id,
                         ScoopIoWorkExecute execute, ScoopIoWorkFinish finish);
void scoop_io_work_retire(ScoopIoWork *work);
void scoop_io_work_finished(ScoopIoWork *work, int status);
void scoop_io_work_cancel(ScoopIoLoop *loop, int64_t id);
ScoopIoWorkTask *scoop_io_work_take(ScoopIoLoop *loop);
void scoop_io_work_run(ScoopIoWorkTask *task);
void scoop_io_work_reject(ScoopIoWorkTask *task);
void scoop_io_work_poll(ScoopIoLoop *loop);

#endif
