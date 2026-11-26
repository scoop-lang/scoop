#pragma once

#include <CoreServices/CoreServices.h>
#include <dispatch/dispatch.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>

typedef struct {
    char *path;
    uint64_t id;
    uint32_t flags;
} DirectoryEvent;

typedef struct EventBatch {
    struct EventBatch *next;
    int64_t count;
    DirectoryEvent events[];
} EventBatch;

typedef struct {
    pthread_mutex_t lock;
    pthread_cond_t changed;
    dispatch_queue_t queue;
    FSEventStreamRef stream;
    EventBatch *head, *tail;
    int error;
    bool reading, closed, stopping;
} DirectoryWatch;

void scoop_os_macos_events(ConstFSEventStreamRef stream, void *info, size_t count, void *paths,
                           const FSEventStreamEventFlags flags[], const FSEventStreamEventId ids[]);
void scoop_os_macos_free_batch(EventBatch *batch);
