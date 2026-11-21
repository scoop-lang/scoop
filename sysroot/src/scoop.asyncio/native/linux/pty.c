#include "../common/pty.h"
#include "../common/pty_unix.h"
#include "backend.h"
#include <poll.h>
#include <sys/syscall.h>

typedef struct {
    ScoopIoRequest request;
    int fd;
    void *bytes;
    int64_t count;
    bool writing, watching_process;
} PtyPoll;

static void prepare(ScoopIoRequest *native, unsigned index, struct io_uring_sqe *sqe) {
    (void)index;
    PtyPoll *request = (void *)native;
    io_uring_prep_poll_add(sqe, request->fd, request->writing ? POLLOUT : POLLIN);
}

static bool try_transfer(PtyPoll *request) {
    return scoop_io_pty_try_transfer(&request->request.completion.event, request->fd,
                                      request->bytes, request->count, request->writing);
}

static void result(ScoopIoRequest *native, unsigned index, int status) {
    if (native->ended)
        return;
    PtyPoll *request = (void *)native;
    if (status < 0)
        scoop_io_request_error(native, -status);
    else if (request->watching_process || try_transfer(request))
        scoop_io_request_end(native);
    else
        scoop_io_request_queue(native, index);
}

static void finish(ScoopIoRequest *native) {
    PtyPoll *request = (void *)native;
    if (request->watching_process)
        (void)close(request->fd);
    scoop_io_complete(native->loop, &native->completion, 0);
}

static void init(PtyPoll *request, ScoopIoLoop *loop, int64_t id) {
    request->request.prepare = prepare;
    request->request.result = result;
    request->request.finish = finish;
    scoop_io_request_init(&request->request, loop, id);
}

int scoop_io_pty_transfer(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, void *bytes,
                          int64_t count, bool writing) {
    PtyPoll *request = calloc(1, sizeof(*request));
    if (!request)
        return -ENOMEM;
    request->fd = scoop_file_handle(writing ? pty->input : pty->output);
    request->bytes = bytes;
    request->count = count;
    request->writing = writing;
    init(request, loop, id);
    if (try_transfer(request))
        scoop_io_request_end(&request->request);
    else
        scoop_io_request_queue(&request->request, 0);
    return 0;
}

ScoopIoStatus scoop_io_pty_watch(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id) {
    int pidfd = (int)syscall(SYS_pidfd_open, (pid_t)pty->process.value, 0U);
    if (pidfd < 0)
        return scoop_io_pty_status(errno);
    PtyPoll *request = calloc(1, sizeof(*request));
    if (!request) {
        (void)close(pidfd);
        return scoop_io_pty_status(ENOMEM);
    }
    request->fd = pidfd;
    request->watching_process = true;
    init(request, loop, id);
    scoop_io_request_queue(&request->request, 0);
    return (ScoopIoStatus){0};
}
