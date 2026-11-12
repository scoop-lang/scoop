#include "../../native/darwin/backend.h"
#include <errno.h>
#include <poll.h>
#include <stdatomic.h>
#include <sys/ioctl.h>

static int notification_count(ScoopIoLoop *loop, int expected) {
    int count = 0;
    if (ioctl(scoop_io_loop_descriptor(loop), FIONREAD, &count) < 0)
        return -errno;
    return count == expected ? 0 : -EPROTO;
}

int scoop_test_darwin_notification_burst(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    if (created.status.code)
        return -created.status.code;
    ScoopIoLoop *loop = created.pointer;
    for (int i = 0; i < 4096; i++)
        scoop_io_loop_wake(loop);
    int status = notification_count(loop, 1);
    if (!status)
        status = -scoop_io_loop_poll(loop, false).code;
    if (!status)
        status = notification_count(loop, 0);
    scoop_io_loop_rearm(loop, true);
    if (!status)
        status = notification_count(loop, 1);
    if (!status)
        status = -scoop_io_loop_poll(loop, true).code;
    if (!status)
        status = notification_count(loop, 0);
    scoop_io_action_begin(loop);
    scoop_io_action_begin(loop);
    scoop_io_action_end(loop);
    if (!status)
        status = notification_count(loop, 0);
    scoop_io_action_end(loop);
    if (!status)
        status = notification_count(loop, 1);
    if (!status)
        status = -scoop_io_loop_poll(loop, true).code;
    if (!status)
        status = notification_count(loop, 0);
    int closed = -scoop_io_loop_destroy(loop).code;
    return status ? status : closed;
}

typedef struct {
    ScoopIoLoop *loop;
    atomic_bool finished;
} WakeProducer;

static void *publish_wakes(void *context) {
    WakeProducer *producer = context;
    for (int i = 0; i < 16384; i++)
        scoop_io_loop_wake(producer->loop);
    atomic_store_explicit(&producer->finished, true, memory_order_release);
    scoop_io_loop_wake(producer->loop);
    return NULL;
}

int scoop_test_darwin_notification_race(void) {
    ScoopIoCreated created = scoop_io_loop_create();
    if (created.status.code)
        return -created.status.code;
    ScoopIoLoop *loop = created.pointer;
    WakeProducer producer = {.loop = loop};
    atomic_init(&producer.finished, false);
    pthread_t thread;
    int status = pthread_create(&thread, NULL, publish_wakes, &producer);
    if (status) {
        scoop_io_loop_destroy(loop);
        return -status;
    }
    while (!atomic_load_explicit(&producer.finished, memory_order_acquire)) {
        struct pollfd descriptor = {.fd = scoop_io_loop_descriptor(loop), .events = POLLIN};
        int ready;
        do {
            ready = poll(&descriptor, 1, 5000);
        } while (ready < 0 && errno == EINTR);
        if (ready <= 0 || !(descriptor.revents & POLLIN)) {
            status = ready < 0 ? -errno : -ETIMEDOUT;
            break;
        }
        status = -scoop_io_loop_poll(loop, false).code;
        if (status)
            break;
    }
    pthread_join(thread, NULL);
    int closed = -scoop_io_loop_destroy(loop).code;
    return status ? status : closed;
}
