#include "native/linux/endpoint.h"
#include <errno.h>
#include <inttypes.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

int scoop_io_tcp_listen(ScoopIoLoop *, int64_t, struct ScoopSocketAddress, int, bool);
int scoop_io_tcp_accept(ScoopIoEndpoint *, ScoopIoLoop *, int64_t);
int scoop_io_tcp_read(ScoopIoEndpoint *, int64_t, void *, int64_t);
int scoop_io_tcp_write(ScoopIoEndpoint *, int64_t, void *, int64_t);
int scoop_io_tcp_close(ScoopIoEndpoint *, int64_t);

typedef struct Connection Connection;
typedef struct {
    ScoopIoLoop *loop;
    pthread_t thread;
    pthread_mutex_t inbox_lock;
    Connection *inbox;
    int index, expected, received, remaining;
} Worker;
struct Connection {
    Connection *next;
    ScoopIoEndpoint *peer;
    unsigned char *buffer;
    int64_t id, length, offset;
    enum { READING, WRITING, CLOSING } phase;
};
static Worker *workers;
static Connection *connections;
static ScoopIoEndpoint *listener;
static int worker_count, connection_count, accepted;
static int64_t buffer_size;
static bool listener_closed;

static void check(int status, const char *operation) {
    if (status < 0) {
        fprintf(stderr, "%s: %d\n", operation, status);
        exit(1);
    }
}
static void read_next(Connection *connection) {
    connection->phase = READING;
    check(scoop_io_tcp_read(connection->peer, connection->id, connection->buffer, buffer_size),
          "read");
}
static void write_next(Connection *connection) {
    connection->phase = WRITING;
    check(scoop_io_tcp_write(connection->peer, connection->id,
                             connection->buffer + connection->offset,
                             connection->length - connection->offset),
          "write");
}
static void accept_next(void) {
    check(scoop_io_tcp_accept(listener, workers[accepted % worker_count].loop, -2), "accept");
}
static void publish(Connection *connection, Worker *target) {
    pthread_mutex_lock(&target->inbox_lock);
    connection->next = target->inbox;
    target->inbox = connection;
    scoop_io_loop_wake(target->loop);
    pthread_mutex_unlock(&target->inbox_lock);
}
static void receive_connections(Worker *worker) {
    if (worker->received == worker->expected)
        return;
    pthread_mutex_lock(&worker->inbox_lock);
    Connection *pending = worker->inbox;
    worker->inbox = NULL;
    pthread_mutex_unlock(&worker->inbox_lock);
    while (pending) {
        Connection *next = pending->next;
        worker->received++;
        read_next(pending);
        pending = next;
    }
}
static void complete(Worker *worker, ScoopIoEvent event) {
    if (event.kind == SCOOP_IO_POSIX_ERROR)
        check(-event.code, "completion");
    if (event.id == -2) {
        if (worker->index || event.kind != SCOOP_IO_RESOURCE || accepted >= connection_count)
            check(-EPROTO, "accept completion");
        Connection *connection = &connections[accepted];
        connection->peer = event.resource;
        Worker *target = &workers[accepted % worker_count];
        accepted++;
        if (target == worker) {
            worker->received++;
            read_next(connection);
        } else {
            publish(connection, target);
        }
        if (accepted == connection_count)
            check(scoop_io_tcp_close(listener, -3), "listener close");
        else
            accept_next();
        return;
    }
    if (event.id == -3) {
        if (event.kind != SCOOP_IO_DONE)
            check(-EPROTO, "listener completion");
        listener_closed = true;
        return;
    }
    if (event.id < 1 || event.id > connection_count ||
        (event.id - 1) % worker_count != worker->index)
        check(-EPROTO, "connection id");
    Connection *connection = &connections[event.id - 1];
    switch (connection->phase) {
    case READING:
        if (event.kind == SCOOP_IO_EOF) {
            connection->phase = CLOSING;
            check(scoop_io_tcp_close(connection->peer, connection->id), "close");
        } else {
            if (event.kind != SCOOP_IO_BYTES || event.count <= 0 || event.count > buffer_size)
                check(-EPROTO, "read completion");
            connection->length = event.count;
            connection->offset = 0;
            write_next(connection);
        }
        break;
    case WRITING:
        if (event.kind != SCOOP_IO_BYTES || event.count <= 0 ||
            event.count > connection->length - connection->offset)
            check(-EPROTO, "write completion");
        connection->offset += event.count;
        if (connection->offset < connection->length)
            write_next(connection);
        else
            read_next(connection);
        break;
    case CLOSING:
        if (event.kind != SCOOP_IO_DONE)
            check(-EPROTO, "close completion");
        connection->peer = NULL;
        worker->remaining--;
        break;
    }
}
static void *drive(void *argument) {
    Worker *worker = argument;
    if (worker->index == 0)
        accept_next();
    while (worker->remaining || (worker->index == 0 && !listener_closed)) {
        receive_connections(worker);
        check(-scoop_io_loop_poll(worker->loop, true).code, "poll");
        ScoopIoBatch batch = scoop_io_loop_take_batch(worker->loop);
        for (int64_t i = 0; i < batch.count; i++)
            complete(worker, batch.records[i]);
        scoop_io_loop_rearm(worker->loop, false);
    }
    check(-scoop_io_loop_destroy(worker->loop).code, "destroy");
    return NULL;
}
static void metrics(void) {
    // These are managed GC counters. Native malloc storage is reflected only by RSS.
    puts("METRICS "
         "{\"allocated_bytes\":0,\"minor_collections\":0,\"full_collections\":0,\"pause_ns\":0,"
         "\"stop_wait_ns\":0}");
    fflush(stdout);
}
int main(int argc, char **argv) {
    if (argc != 5)
        return 2;
    worker_count = atoi(argv[1]);
    int port = atoi(argv[2]);
    connection_count = atoi(argv[3]);
    buffer_size = strtoll(argv[4], NULL, 10);
    if (worker_count < 1 || worker_count > 64 || connection_count < 1 || buffer_size < 1)
        return 2;
    workers = calloc((size_t)worker_count, sizeof(*workers));
    connections = calloc((size_t)connection_count, sizeof(*connections));
    if (!workers || !connections)
        return 1;
    metrics();
    for (int i = 0; i < connection_count; i++) {
        connections[i].id = i + 1;
        connections[i].buffer = malloc((size_t)buffer_size);
        if (!connections[i].buffer)
            return 1;
        workers[i % worker_count].expected++;
    }
    for (int i = 0; i < worker_count; i++) {
        Worker *worker = &workers[i];
        worker->index = i;
        worker->remaining = worker->expected;
        check(-pthread_mutex_init(&worker->inbox_lock, NULL), "mutex init");
        ScoopIoCreated created = scoop_io_loop_create();
        check(-created.status.code, "loop create");
        worker->loop = created.pointer;
    }
    struct ScoopSocketAddress address = {.family = 4, .port = port, .a = 0x7f000001};
    check(scoop_io_tcp_listen(workers[0].loop, -1, address, 1024, true), "listen");
    check(-scoop_io_loop_poll(workers[0].loop, false).code, "listen poll");
    ScoopIoBatch batch = scoop_io_loop_take_batch(workers[0].loop);
    if (batch.count != 1 || batch.records[0].kind != SCOOP_IO_RESOURCE)
        check(-EPROTO, "listen completion");
    listener = batch.records[0].resource;
    scoop_io_loop_rearm(workers[0].loop, false);
    int enabled = 1;
    if (setsockopt(listener->fd, IPPROTO_TCP, TCP_NODELAY, &enabled, sizeof(enabled)))
        check(-errno, "TCP_NODELAY");
    for (int i = 0; i < worker_count; i++)
        check(-pthread_create(&workers[i].thread, NULL, drive, &workers[i]), "thread create");
    puts("READY");
    fflush(stdout);
    for (int i = 0; i < worker_count; i++) {
        check(-pthread_join(workers[i].thread, NULL), "join");
        pthread_mutex_destroy(&workers[i].inbox_lock);
    }
    for (int i = 0; i < connection_count; i++)
        free(connections[i].buffer);
    free(connections);
    free(workers);
    metrics();
    return 0;
}
