#include "watch.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>

void scoop_os_macos_free_batch(EventBatch *batch) {
    for (int64_t i = 0; i < batch->count; ++i)
        free(batch->events[i].path);
    free(batch);
}

static EventBatch *copy_events(size_t count, char **paths, const FSEventStreamEventFlags *flags,
                               const FSEventStreamEventId *ids) {
    if (count > (SIZE_MAX - sizeof(EventBatch)) / sizeof(DirectoryEvent)) {
        errno = EOVERFLOW;
        return NULL;
    }
    EventBatch *batch = calloc(1, sizeof(*batch) + count * sizeof(batch->events[0]));
    if (!batch)
        return NULL;
    for (size_t i = 0; i < count; ++i) {
        char *path = strdup(paths[i]);
        if (!path) {
            scoop_os_macos_free_batch(batch);
            errno = ENOMEM;
            return NULL;
        }
        batch->events[i] = (DirectoryEvent){path, ids[i], flags[i]};
        ++batch->count;
    }
    return batch;
}

void scoop_os_macos_events(ConstFSEventStreamRef stream, void *info, size_t count, void *paths,
                           const FSEventStreamEventFlags flags[], const FSEventStreamEventId ids[]) {
    (void)stream;
    if (!count)
        return;
    DirectoryWatch *watch = info;
    EventBatch *batch = copy_events(count, paths, flags, ids);
    int error = batch ? 0 : errno;
    pthread_mutex_lock(&watch->lock);
    if (!watch->closed && !watch->error) {
        if (batch) {
            if (watch->tail)
                watch->tail->next = batch;
            else
                watch->head = batch;
            watch->tail = batch;
            batch = NULL;
        } else {
            watch->error = error;
        }
        pthread_cond_signal(&watch->changed);
    }
    pthread_mutex_unlock(&watch->lock);
    if (batch)
        scoop_os_macos_free_batch(batch);
}

typedef struct {
    EventBatch *batch;
    int64_t count;
    int32_t error;
} ReadResult;

ReadResult scoop_os_macos_next_batch(DirectoryWatch *watch) {
    ReadResult result = {0};
    pthread_mutex_lock(&watch->lock);
    if (!watch->closed) {
        if (watch->reading) {
            result.error = -1;
        } else {
            watch->reading = true;
            while (!watch->closed && !watch->head && !watch->error)
                pthread_cond_wait(&watch->changed, &watch->lock);
            if (!watch->closed && watch->head) {
                result.batch = watch->head;
                result.count = result.batch->count;
                watch->head = result.batch->next;
                if (!watch->head)
                    watch->tail = NULL;
            } else {
                if (!watch->closed)
                    result.error = watch->error;
                watch->reading = false;
            }
        }
    }
    pthread_mutex_unlock(&watch->lock);
    return result;
}

DirectoryEvent scoop_os_macos_batch_event(EventBatch *batch, int64_t index) {
    return batch->events[index];
}

void scoop_os_macos_finish_read(DirectoryWatch *watch, EventBatch *batch) {
    scoop_os_macos_free_batch(batch);
    pthread_mutex_lock(&watch->lock);
    watch->reading = false;
    pthread_mutex_unlock(&watch->lock);
}
