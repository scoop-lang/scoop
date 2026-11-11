#ifndef SCOOP_ASYNCIO_LOOP_H
#define SCOOP_ASYNCIO_LOOP_H

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct ScoopIoEvent {
    int64_t id;
    int kind;
    int code;
    int64_t count;
    void *resource;
} ScoopIoEvent;

enum {
    SCOOP_IO_DONE,
    SCOOP_IO_BYTES,
    SCOOP_IO_EOF,
    SCOOP_IO_RESOURCE,
    SCOOP_IO_POSIX_ERROR = 5,
    SCOOP_IO_ADDRESSES,
    SCOOP_IO_ADDRESS_ERROR,
    SCOOP_IO_DATAGRAM,
    SCOOP_IO_WIN32_ERROR,
    SCOOP_IO_WINSOCK_ERROR,
    SCOOP_IO_NTSTATUS_ERROR,
    /* File and directory events occupy 0x100..0x1ff. */
    SCOOP_IO_METADATA = 0x100,
    SCOOP_IO_POSITION = 0x101,
    SCOOP_IO_TEMPORARY = 0x102,
    SCOOP_IO_DIRECTORY_NAME = 0x103,
    /* TTY events occupy 0x200..0x2ff. */
    SCOOP_IO_TTY_RESIZE = 0x200,
    SCOOP_IO_TTY_TIMEOUT = 0x201
};

typedef struct ScoopIoCompletion {
    ScoopIoEvent event;
    struct ScoopIoCompletion *next;
} ScoopIoCompletion;

enum { SCOOP_IO_BATCH_CAPACITY = 64 };
typedef struct ScoopIoBatch {
    const ScoopIoEvent *records;
    int64_t count;
} ScoopIoBatch;

typedef struct {
    void *endpoint;
    int64_t id;
    void *bytes;
    int64_t count;
    bool writing;
} ScoopIoTcpSubmission;

typedef struct {
    ScoopIoTcpSubmission entries[SCOOP_IO_BATCH_CAPACITY];
    ScoopIoEvent failures[SCOOP_IO_BATCH_CAPACITY];
    unsigned count;
} ScoopIoTcpSubmissions;

typedef struct ScoopIoLoop ScoopIoLoop;

typedef struct ScoopIoStatus {
    int domain, code;
} ScoopIoStatus;

static inline ScoopIoStatus scoop_io_errno(int status) {
    return (ScoopIoStatus){SCOOP_IO_POSIX_ERROR, -status};
}

typedef struct ScoopIoCreated {
    void *pointer;
    ScoopIoStatus status;
} ScoopIoCreated;

void scoop_io_complete(ScoopIoLoop *loop, ScoopIoCompletion *completion, int status);
ScoopIoCreated scoop_io_loop_create(void);
void scoop_io_loop_wake(ScoopIoLoop *loop);
ScoopIoStatus scoop_io_loop_poll(ScoopIoLoop *loop, bool wait);
ScoopIoBatch scoop_io_loop_take_batch(ScoopIoLoop *loop);
void scoop_io_loop_rearm(ScoopIoLoop *loop, bool managed_ready);
bool scoop_io_tcp_enqueue(ScoopIoLoop *loop, void *endpoint, int64_t id, void *bytes,
                           int64_t count, bool writing);
ScoopIoBatch scoop_io_tcp_flush(ScoopIoLoop *loop);
bool scoop_io_loop_idle(ScoopIoLoop *loop);
#ifdef _WIN32
void *scoop_io_loop_event(ScoopIoLoop *loop);
#else
int scoop_io_loop_descriptor(ScoopIoLoop *loop);
#endif
ScoopIoStatus scoop_io_loop_destroy(ScoopIoLoop *loop);
ScoopIoStatus scoop_io_timer_start(ScoopIoLoop *loop, int64_t id, int64_t seconds, int nanos);
void scoop_io_timer_cancel(ScoopIoLoop *loop, int64_t id);

#endif
