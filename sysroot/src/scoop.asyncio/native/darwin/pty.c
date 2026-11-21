#include "../common/pty.h"
#include "../common/pty_unix.h"
#include "wait.h"
#include <sys/wait.h>

typedef struct {
    ScoopIoWait wait;
    int fd;
    void *bytes;
    int64_t count;
    bool writing;
} PtyTransfer;

typedef struct {
    ScoopIoWait wait;
    pid_t pid;
} PtyWatch;

static void finished(ScoopIoWait *wait) { scoop_io_request_finish(&wait->request); }

static void cancel(ScoopIoRequest *request) { scoop_io_wait_end((ScoopIoWait *)request); }

static bool try_transfer(PtyTransfer *request) {
    return scoop_io_pty_try_transfer(&request->wait.request.completion.event, request->fd,
                                      request->bytes, request->count, request->writing);
}

static void transfer_ready(void *context) {
    PtyTransfer *request = context;
    if (!request->wait.ended && try_transfer(request))
        scoop_io_wait_end(&request->wait);
}

static void transfer_started(void *context) {
    PtyTransfer *request = context;
    ScoopIoLoop *loop = request->wait.request.loop;
    if (!scoop_io_request_start(&request->wait.request) || try_transfer(request)) {
        scoop_io_wait_end(&request->wait);
    } else {
        int status = scoop_io_wait_fd(&request->wait, request->fd, request->writing, transfer_ready);
        if (status < 0)
            scoop_io_wait_error(&request->wait, -status);
    }
    scoop_io_action_end(loop);
}

int scoop_io_pty_transfer(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, void *bytes,
                          int64_t count, bool writing) {
    PtyTransfer *request = calloc(1, sizeof(*request));
    if (!request)
        return -ENOMEM;
    request->wait.finish = finished;
    request->fd = scoop_file_handle(writing ? pty->input : pty->output);
    request->bytes = bytes;
    request->count = count;
    request->writing = writing;
    scoop_io_request_submit(&request->wait.request, loop, id, transfer_started, cancel);
    return 0;
}

static void watch_ready(void *context) { scoop_io_wait_end(context); }

static void watch_started(void *context) {
    PtyWatch *watch = context;
    ScoopIoLoop *loop = watch->wait.request.loop;
    if (!scoop_io_request_start(&watch->wait.request)) {
        scoop_io_wait_end(&watch->wait);
    } else {
        int status = scoop_io_wait_process(&watch->wait, watch->pid, watch_ready);
        // Detect a child which became waitable before the source was registered.
        siginfo_t info = {0};
        int observed;
        do {
            observed = waitid(P_PID, (id_t)watch->pid, &info, WNOHANG | WEXITED | WNOWAIT);
        } while (observed < 0 && errno == EINTR);
        if (observed < 0)
            scoop_io_wait_error(&watch->wait, errno);
        else if (info.si_pid)
            scoop_io_wait_end(&watch->wait);
        else if (status < 0)
            scoop_io_wait_error(&watch->wait, -status);
    }
    scoop_io_action_end(loop);
}

ScoopIoStatus scoop_io_pty_watch(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id) {
    PtyWatch *watch = calloc(1, sizeof(*watch));
    if (!watch)
        return scoop_io_pty_status(ENOMEM);
    watch->wait.finish = finished;
    watch->pid = (pid_t)pty->process.value;
    scoop_io_request_submit(&watch->wait.request, loop, id, watch_started, cancel);
    return (ScoopIoStatus){0};
}
