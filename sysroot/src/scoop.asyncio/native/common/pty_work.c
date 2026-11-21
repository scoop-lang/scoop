#include "platform.h"
#include "pty.h"
#include <errno.h>

#if defined(_WIN32)
#include "../windows/backend.h"
#elif defined(__APPLE__)
#include "../darwin/backend.h"
#else
#include "../linux/backend.h"
#endif

typedef enum { PTY_RESIZE, PTY_END_CONSOLE, PTY_CLOSE } PtyControl;

typedef struct {
    ScoopIoWork work;
    ScoopPlatformPty *pty;
    PtyControl control;
    int32_t columns, rows;
} PtyWork;

static void execute(ScoopIoWork *work) {
    PtyWork *request = (void *)work;
    int error = 0;
    switch (request->control) {
    case PTY_RESIZE:
        error = scoop_platform_pty_resize(request->pty, request->columns, request->rows);
        break;
    case PTY_END_CONSOLE:
#ifdef _WIN32
        scoop_platform_pty_close_console(request->pty);
#endif
        break;
    case PTY_CLOSE:
        error = scoop_platform_pty_close(request->pty);
        break;
    }
    scoop_io_platform_error(&work->completion.event, error);
}

static void finish(ScoopIoWork *work, int status) {
    PtyWork *request = (void *)work;
    if (status && request->control == PTY_CLOSE)
        (void)scoop_platform_pty_close(request->pty);
    scoop_io_work_finished(work, status);
}

static int submit(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, PtyControl control,
                  int32_t columns, int32_t rows) {
    PtyWork *request = calloc(1, sizeof(*request));
    if (!request) {
        if (control == PTY_CLOSE)
            (void)scoop_platform_pty_close(pty);
        return -ENOMEM;
    }
    request->pty = pty;
    request->control = control;
    request->columns = columns;
    request->rows = rows;
    int status = scoop_io_work_submit(&request->work, loop, id, execute, finish);
    if (status < 0) {
        if (control == PTY_CLOSE)
            (void)scoop_platform_pty_close(pty);
        free(request);
    }
    return status;
}

int scoop_io_pty_resize(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, int32_t columns,
                        int32_t rows) {
    return submit(loop, pty, id, PTY_RESIZE, columns, rows);
}

int scoop_io_pty_end_console(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id) {
    return submit(loop, pty, id, PTY_END_CONSOLE, 0, 0);
}

int scoop_io_pty_close(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id) {
    return submit(loop, pty, id, PTY_CLOSE, 0, 0);
}

ScoopIoStatus scoop_io_pty_close_output(ScoopPlatformPty *pty) {
#ifdef _WIN32
    return scoop_io_pty_status(scoop_platform_pty_close_output(pty));
#else
    (void)pty;
    return (ScoopIoStatus){0};
#endif
}

void scoop_io_pty_cancel(ScoopIoLoop *loop, int64_t id) {
    scoop_io_request_cancel(loop, id);
}

#ifndef _WIN32
ScoopIoStatus scoop_io_pty_associate(ScoopIoLoop *loop, ScoopPlatformPty *pty) {
    (void)loop;
    (void)pty;
    return (ScoopIoStatus){0};
}
#endif
