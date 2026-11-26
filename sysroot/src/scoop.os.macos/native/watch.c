#include "watch.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    DirectoryWatch *watch;
    int32_t error;
} OpenResult;

static FSEventStreamRef create_stream(const char *path, DirectoryWatch *watch) {
    CFURLRef url =
        CFURLCreateFromFileSystemRepresentation(NULL, (const UInt8 *)path, strlen(path), true);
    if (!url)
        return NULL;
    CFURLRef absolute = CFURLCopyAbsoluteURL(url);
    CFRelease(url);
    if (!absolute)
        return NULL;
    CFStringRef string = CFURLCopyFileSystemPath(absolute, kCFURLPOSIXPathStyle);
    CFRelease(absolute);
    if (!string)
        return NULL;
    const void *values[] = {string};
    CFArrayRef paths = CFArrayCreate(NULL, values, 1, &kCFTypeArrayCallBacks);
    CFRelease(string);
    if (!paths)
        return NULL;
    FSEventStreamContext context = {.info = watch};
    FSEventStreamRef stream =
        FSEventStreamCreate(NULL, scoop_os_macos_events, &context, paths, kFSEventStreamEventIdSinceNow,
                            0.05, kFSEventStreamCreateFlagFileEvents);
    CFRelease(paths);
    return stream;
}

static void barrier(void *unused) { (void)unused; }

static void release_stream(DirectoryWatch *watch) {
    FSEventStreamInvalidate(watch->stream);
    dispatch_sync_f(watch->queue, NULL, barrier);
    FSEventStreamRelease(watch->stream);
    watch->stream = NULL;
}

OpenResult scoop_os_macos_watch_open(const char *path) {
    DirectoryWatch *watch = calloc(1, sizeof(*watch));
    if (!watch)
        return (OpenResult){NULL, ENOMEM};
    int error = pthread_mutex_init(&watch->lock, NULL);
    if (error)
        goto free_watch;
    error = pthread_cond_init(&watch->changed, NULL);
    if (error)
        goto free_lock;
    watch->queue = dispatch_queue_create("scoop.os.macos.fsevents", DISPATCH_QUEUE_SERIAL);
    if (!watch->queue) {
        error = ENOMEM;
        goto free_condition;
    }
    watch->stream = create_stream(path, watch);
    if (watch->stream) {
        FSEventStreamSetDispatchQueue(watch->stream, watch->queue);
        if (FSEventStreamStart(watch->stream))
            return (OpenResult){watch, 0};
        release_stream(watch);
    }
    dispatch_release(watch->queue);
free_condition:
    pthread_cond_destroy(&watch->changed);
free_lock:
    pthread_mutex_destroy(&watch->lock);
free_watch:
    free(watch);
    return (OpenResult){NULL, error};
}

bool scoop_os_macos_watch_closed(DirectoryWatch *watch) {
    pthread_mutex_lock(&watch->lock);
    bool closed = watch->closed;
    pthread_mutex_unlock(&watch->lock);
    return closed;
}

void scoop_os_macos_watch_close(DirectoryWatch *watch) {
    pthread_mutex_lock(&watch->lock);
    if (watch->closed) {
        while (watch->stopping)
            pthread_cond_wait(&watch->changed, &watch->lock);
        pthread_mutex_unlock(&watch->lock);
        return;
    }
    watch->closed = true;
    watch->stopping = true;
    EventBatch *discard = watch->head;
    watch->head = watch->tail = NULL;
    pthread_cond_broadcast(&watch->changed);
    pthread_mutex_unlock(&watch->lock);

    FSEventStreamStop(watch->stream);
    release_stream(watch);
    dispatch_release(watch->queue);
    while (discard) {
        EventBatch *next = discard->next;
        scoop_os_macos_free_batch(discard);
        discard = next;
    }

    pthread_mutex_lock(&watch->lock);
    watch->stopping = false;
    pthread_cond_broadcast(&watch->changed);
    pthread_mutex_unlock(&watch->lock);
}

static void destroy(void *owner) {
    DirectoryWatch *watch = owner;
    scoop_os_macos_watch_close(watch);
    pthread_cond_destroy(&watch->changed);
    pthread_mutex_destroy(&watch->lock);
    free(watch);
}

void scoop_os_macos_watch_release(DirectoryWatch *watch) {
    // GC cleanup never waits for FSEvents or a dispatch queue.
    dispatch_async_f(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), watch, destroy);
}
