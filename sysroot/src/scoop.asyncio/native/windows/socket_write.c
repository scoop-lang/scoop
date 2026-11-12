#include "socket.h"
#include <errno.h>
#include <limits.h>

typedef struct {
    ScoopIoRequest request;
    ScoopIoSocket *socket;
    void *bytes;
    int count;
    WSAEVENT event;
} SocketWrite;

static void finish(SocketWrite *write, DWORD count, ScoopIoStatus status) {
    if (write->event) {
        if (WSAEventSelect(write->socket->socket, NULL, 0) != 0 && !status.code)
            status = scoop_io_winsock(WSAGetLastError());
        if (!WSACloseEvent(write->event) && !status.code)
            status = scoop_io_winsock(WSAGetLastError());
    }
    scoop_io_tcp_result(write->socket, &write->request.completion.event, count, status, true);
    scoop_io_socket_finished(&write->request, write->socket, true);
}

static VOID CALLBACK ready(PTP_CALLBACK_INSTANCE instance, void *context, PTP_WAIT wait,
                           TP_WAIT_RESULT result) {
    (void)instance;
    (void)result;
    SocketWrite *write = context;
    ScoopIoLoop *loop = write->request.loop;
    // A wait is one-shot. Its storage is freed after this outstanding callback returns.
    CloseThreadpoolWait(wait);
    AcquireSRWLockExclusive(&loop->mutex);
    scoop_io_packet_locked(&write->request, 0, 0);
    // No access to write or loop follows publication under this lock.
    ReleaseSRWLockExclusive(&loop->mutex);
}

static void attempt(SocketWrite *write) {
    if (write->request.cancelled) {
        finish(write, 0, scoop_io_winsock(WSA_OPERATION_ABORTED));
        return;
    }
    for (;;) {
        if (write->event) {
            WSANETWORKEVENTS events;
            if (WSAEnumNetworkEvents(write->socket->socket, write->event, &events) != 0) {
                finish(write, 0, scoop_io_winsock(WSAGetLastError()));
                return;
            }
            int error = (events.lNetworkEvents & FD_WRITE) ? events.iErrorCode[FD_WRITE_BIT] : 0;
            if (!error && (events.lNetworkEvents & FD_CLOSE))
                error = events.iErrorCode[FD_CLOSE_BIT];
            if (error) {
                finish(write, 0, scoop_io_winsock(error));
                return;
            }
        }
        int count = send(write->socket->socket, write->bytes, write->count, 0);
        if (count != SOCKET_ERROR) {
            finish(write, (DWORD)count, (ScoopIoStatus){0});
            return;
        }
        int error = WSAGetLastError();
        if (error != WSAEWOULDBLOCK) {
            finish(write, 0, scoop_io_winsock(error));
            return;
        }
        if (!write->event) {
            write->event = WSACreateEvent();
            if (!write->event ||
                WSAEventSelect(write->socket->socket, write->event, FD_WRITE | FD_CLOSE) != 0) {
                finish(write, 0, scoop_io_winsock(WSAGetLastError()));
                return;
            }
            // Subscribe, discard the initial event, then retry before installing the wait.
            continue;
        }
        PTP_WAIT wait = CreateThreadpoolWait(ready, write, NULL);
        if (!wait) {
            finish(write, 0, scoop_io_win32(GetLastError()));
            return;
        }
        SetThreadpoolWait(wait, write->event, NULL);
        return;
    }
}

static void packet(ScoopIoRequest *request, DWORD count, DWORD error) {
    (void)count;
    (void)error;
    attempt((SocketWrite *)request);
}

static void cancel(ScoopIoRequest *request) {
    SocketWrite *write = (SocketWrite *)request;
    if (!WSASetEvent(write->event)) {
        int error = WSAGetLastError();
        AcquireSRWLockExclusive(&request->loop->mutex);
        scoop_io_fail_locked(request->loop, scoop_io_winsock(error));
        scoop_io_notify_locked(request->loop);
        ReleaseSRWLockExclusive(&request->loop->mutex);
    }
}

int scoop_io_tcp_write(ScoopIoSocket *socket, int64_t id, void *bytes, int64_t count) {
    SocketWrite *write = calloc(1, sizeof(*write));
    if (!write)
        return -ENOMEM;
    write->socket = socket;
    write->bytes = bytes;
    write->count = count > INT_MAX ? INT_MAX : (int)count;
    scoop_io_request_init(&write->request, socket->loop, id, (HANDLE)socket->socket, packet);
    write->request.cancel = cancel;
    socket->active++;
    socket->output = &write->request;
    attempt(write);
    return 0;
}
