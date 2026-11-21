#include "backend.h"
#include "../common/platform.h"
#include "../common/pty.h"
#include <errno.h>

typedef struct {
    ScoopIoRequest request;
    bool writing;
} PtyTransfer;

typedef struct {
    ScoopIoRequest request;
    PTP_WAIT wait;
    PTP_WORK stop;
    // Accessed by the owner and the thread-pool callbacks under the loop mutex.
    bool stopping, published;
} PtyWatch;

ScoopIoStatus scoop_io_pty_associate(ScoopIoLoop *loop, ScoopPlatformPty *pty) {
    if (!CreateIoCompletionPort(scoop_file_handle(pty->output), loop->port, 0, 0) ||
        !CreateIoCompletionPort(scoop_file_handle(pty->input), loop->port, 0, 0))
        return scoop_io_win32(GetLastError());
    return (ScoopIoStatus){0};
}

static void transfer_finished(ScoopIoRequest *native, DWORD count, DWORD error) {
    PtyTransfer *transfer = (void *)native;
    ScoopIoEvent *event = &native->completion.event;
    if (!transfer->writing && (error == ERROR_HANDLE_EOF || error == ERROR_BROKEN_PIPE)) {
        event->kind = SCOOP_IO_EOF;
    } else {
        event->kind = !transfer->writing && !count ? SCOOP_IO_EOF : SCOOP_IO_BYTES;
        event->count = count;
        scoop_io_platform_error(event, (int)error);
    }
    scoop_io_request_retire(native);
    scoop_io_complete(native->loop, &native->completion, 0);
}

int scoop_io_pty_transfer(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, void *bytes,
                          int64_t count, bool writing) {
    PtyTransfer *transfer = calloc(1, sizeof(*transfer));
    if (!transfer)
        return -ENOMEM;
    transfer->writing = writing;
    HANDLE handle = scoop_file_handle(writing ? pty->input : pty->output);
    scoop_io_request_init(&transfer->request, loop, id, handle, transfer_finished);
    OVERLAPPED *overlapped = &transfer->request.overlapped;
    DWORD length = (uint64_t)count > UINT32_MAX ? UINT32_MAX : (DWORD)count;
    BOOL issued = writing ? WriteFile(handle, bytes, length, NULL, overlapped)
                          : ReadFile(handle, bytes, length, NULL, overlapped);
    if (!issued) {
        DWORD error = GetLastError();
        if (error != ERROR_IO_PENDING)
            transfer_finished(&transfer->request, 0, error);
    }
    return 0;
}

static VOID CALLBACK watch_ready(PTP_CALLBACK_INSTANCE instance, void *context, PTP_WAIT wait,
                                 TP_WAIT_RESULT result) {
    (void)instance;
    (void)wait;
    (void)result;
    PtyWatch *watch = context;
    ScoopIoLoop *loop = watch->request.loop;
    AcquireSRWLockExclusive(&loop->mutex);
    if (!watch->stopping) {
        watch->published = true;
        scoop_io_packet_locked(&watch->request, 0, 0);
    }
    ReleaseSRWLockExclusive(&loop->mutex);
}

static VOID CALLBACK watch_stop(PTP_CALLBACK_INSTANCE instance, void *context, PTP_WORK work) {
    (void)instance;
    (void)work;
    PtyWatch *watch = context;
    ScoopIoLoop *loop = watch->request.loop;
    SetThreadpoolWait(watch->wait, NULL, NULL);
    WaitForThreadpoolWaitCallbacks(watch->wait, TRUE);
    AcquireSRWLockExclusive(&loop->mutex);
    scoop_io_packet_locked(&watch->request, 0, ERROR_OPERATION_ABORTED);
    ReleaseSRWLockExclusive(&loop->mutex);
}

static void watch_cancel(ScoopIoRequest *native) {
    PtyWatch *watch = (void *)native;
    AcquireSRWLockExclusive(&native->loop->mutex);
    if (!watch->published) {
        watch->stopping = true;
        SubmitThreadpoolWork(watch->stop);
    }
    ReleaseSRWLockExclusive(&native->loop->mutex);
}

static void watch_finished(ScoopIoRequest *native, DWORD count, DWORD error) {
    (void)count;
    PtyWatch *watch = (void *)native;
    CloseThreadpoolWait(watch->wait);
    CloseThreadpoolWork(watch->stop);
    scoop_io_platform_error(&native->completion.event, (int)error);
    scoop_io_request_retire(native);
    scoop_io_complete(native->loop, &native->completion, 0);
}

ScoopIoStatus scoop_io_pty_watch(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id) {
    PtyWatch *watch = calloc(1, sizeof(*watch));
    if (!watch)
        return scoop_io_errno(-ENOMEM);
    watch->wait = CreateThreadpoolWait(watch_ready, watch, NULL);
    if (watch->wait)
        watch->stop = CreateThreadpoolWork(watch_stop, watch, NULL);
    if (!watch->wait || !watch->stop) {
        DWORD error = GetLastError();
        if (watch->wait)
            CloseThreadpoolWait(watch->wait);
        free(watch);
        return scoop_io_win32(error);
    }
    HANDLE handle = (HANDLE)(uintptr_t)pty->process.value;
    scoop_io_request_init(&watch->request, loop, id, handle, watch_finished);
    watch->request.cancel = watch_cancel;
    SetThreadpoolWait(watch->wait, handle, NULL);
    return (ScoopIoStatus){0};
}
