#include "socket.h"
#include <errno.h>
#include <limits.h>

typedef enum { TCP_READ, TCP_SHUTDOWN, UDP_READ, UDP_WRITE } SocketAction;
typedef struct {
    ScoopIoRequest request;
    ScoopIoSocket *socket;
    SocketAction action;
    WSABUF buffer;
    DWORD flags, transferred;
    struct sockaddr_storage address;
    int address_length;
    struct ScoopDatagram *datagram;
} SocketIo;

static bool writing(SocketAction action) { return action == TCP_SHUTDOWN || action == UDP_WRITE; }

void scoop_io_tcp_result(ScoopIoSocket *socket, ScoopIoEvent *event, DWORD count,
                         ScoopIoStatus status, bool output) {
    event->count = count;
    if (status.code) {
        if (!output && status.code != WSA_OPERATION_ABORTED)
            socket->read_error = status;
        scoop_io_socket_error(event, status);
    } else if (!output && count == 0) {
        socket->eof = true;
        event->kind = SCOOP_IO_EOF;
    } else
        event->kind = SCOOP_IO_BYTES;
}

static void finish(SocketIo *request, DWORD count, ScoopIoStatus status, DWORD flags) {
    ScoopIoEvent *event = &request->request.completion.event;
    ScoopIoSocket *socket = request->socket;
    bool truncated = request->action == UDP_READ && status.domain == SCOOP_IO_WINSOCK_ERROR &&
                     status.code == WSAEMSGSIZE;
    if (truncated) {
        count = request->buffer.len;
        status.code = 0;
    }
    event->count = count;
    if (request->action == TCP_READ)
        scoop_io_tcp_result(socket, event, count, status, false);
    else if (!status.code && request->action == UDP_READ) {
        request->datagram->source = scoop_socket_address_decode(
            (struct sockaddr *)&request->address, request->address_length);
        if (!request->datagram->source.family)
            status = scoop_io_winsock(WSAGetLastError());
        else {
            request->datagram->count = count;
            request->datagram->truncated = truncated || (flags & MSG_PARTIAL) != 0;
            event->kind = SCOOP_IO_DATAGRAM;
            event->resource = request->datagram;
        }
    } else if (!status.code && request->action != TCP_SHUTDOWN)
        event->kind = SCOOP_IO_BYTES;
    if (request->action != TCP_READ)
        scoop_io_socket_error(event, status);
    if (event->kind != SCOOP_IO_DATAGRAM)
        free(request->datagram);
    scoop_io_socket_finished(&request->request, socket, writing(request->action));
}

static void packet(ScoopIoRequest *native, DWORD count, DWORD error) {
    (void)error;
    DWORD flags = 0;
    ScoopIoStatus status = scoop_io_socket_result(native, &count, &flags);
    finish((void *)native, count, status, flags);
}

static int submit(ScoopIoSocket *socket, int64_t id, SocketAction action, void *bytes,
                  int64_t count, struct ScoopSocketAddress address, ScoopIoEvent *immediate) {
    SocketIo *request = calloc(1, sizeof(*request));
    if (!request)
        return -ENOMEM;
    if (action == UDP_READ) {
        request->datagram = calloc(1, sizeof(*request->datagram));
        if (!request->datagram) {
            free(request);
            return -ENOMEM;
        }
    }
    request->socket = socket;
    request->action = action;
    request->buffer.buf = bytes;
    request->buffer.len = count > INT_MAX ? INT_MAX : (ULONG)count;
    request->address_length = sizeof(request->address);
    scoop_io_request_init(&request->request, socket->loop, id, (HANDLE)socket->socket, packet);
    socket->active++;
    if (writing(action))
        socket->output = &request->request;
    else
        socket->input = &request->request;
    int status = 0;
    switch (action) {
    case TCP_READ:
        if (socket->eof || socket->read_error.code) {
            finish(request, 0, socket->read_error, 0);
            return 0;
        }
        status = WSARecv(socket->socket, &request->buffer, 1,
                         socket->skip_read_packet ? &request->transferred : NULL, &request->flags,
                         &request->request.overlapped, NULL);
        break;
    case TCP_SHUTDOWN:
        status = shutdown(socket->socket, SD_SEND);
        finish(request, 0, scoop_io_winsock(status ? WSAGetLastError() : 0), 0);
        return 0;
    case UDP_READ:
        status = WSARecvFrom(socket->socket, &request->buffer, 1, NULL, &request->flags,
                             (struct sockaddr *)&request->address, &request->address_length,
                             &request->request.overlapped, NULL);
        break;
    case UDP_WRITE:
        if (count > INT_MAX) {
            finish(request, 0, scoop_io_winsock(WSAEMSGSIZE), 0);
            return 0;
        }
        request->address_length = scoop_socket_address_encode(address, &request->address);
        if (!request->address_length) {
            finish(request, 0, scoop_io_winsock(WSAGetLastError()), 0);
            return 0;
        }
        status = WSASendTo(socket->socket, &request->buffer, 1, NULL, 0,
                           (struct sockaddr *)&request->address, request->address_length,
                           &request->request.overlapped, NULL);
        break;
    }
    if (status == 0 && action == TCP_READ && socket->skip_read_packet) {
        DWORD transferred = request->transferred;
        if (!immediate) {
            finish(request, transferred, (ScoopIoStatus){0}, request->flags);
            return 0;
        }
        // This handle suppresses the success packet, so the native request can retire now.
        scoop_io_tcp_result(socket, &request->request.completion.event, transferred,
                            (ScoopIoStatus){0}, false);
        *immediate = request->request.completion.event;
        scoop_io_request_retire(&request->request);
        socket->input = NULL;
        socket->active--;
        free(request);
        return 1;
    }
    if (status != 0) {
        int error = WSAGetLastError();
        if (error != WSA_IO_PENDING)
            finish(request, 0, scoop_io_winsock(error), request->flags);
    }
    return 0;
}

// The owner holds this direction until managed ready delivery ends the current borrow.
int scoop_io_tcp_transfer(ScoopIoSocket *socket, int64_t id, void *bytes, int64_t length,
                          bool output, ScoopIoEvent *event) {
    *event = (ScoopIoEvent){.id = id};
    if (!output) {
        if (socket->eof || socket->read_error.code) {
            scoop_io_tcp_result(socket, event, 0, socket->read_error, false);
            return 1;
        }
        return submit(socket, id, TCP_READ, bytes, length, (struct ScoopSocketAddress){0}, event);
    }
    int prefix = length > INT_MAX ? INT_MAX : (int)length;
    int count = send(socket->socket, bytes, prefix, 0);
    ScoopIoStatus status = {0};
    if (count == SOCKET_ERROR) {
        int error = WSAGetLastError();
        if (error == WSAEWOULDBLOCK)
            return scoop_io_tcp_write(socket, id, bytes, length);
        status = scoop_io_winsock(error);
        count = 0;
    }
    scoop_io_tcp_result(socket, event, (DWORD)count, status, true);
    return 1;
}

int scoop_io_tcp_read(ScoopIoSocket *socket, int64_t id, void *bytes, int64_t count) {
    return submit(socket, id, TCP_READ, bytes, count, (struct ScoopSocketAddress){0}, NULL);
}
int scoop_io_tcp_shutdown(ScoopIoSocket *socket, int64_t id) {
    return submit(socket, id, TCP_SHUTDOWN, NULL, 0, (struct ScoopSocketAddress){0}, NULL);
}
int scoop_io_udp_receive(ScoopIoSocket *socket, int64_t id, void *bytes, int64_t count) {
    return submit(socket, id, UDP_READ, bytes, count, (struct ScoopSocketAddress){0}, NULL);
}
int scoop_io_udp_send(ScoopIoSocket *socket, int64_t id, void *bytes, int64_t count,
                      struct ScoopSocketAddress address) {
    return submit(socket, id, UDP_WRITE, bytes, count, address, NULL);
}
void scoop_io_read_cancel(ScoopIoSocket *socket, int64_t id) {
    scoop_io_request_cancel(socket->loop, id);
}
void scoop_io_tcp_write_cancel(ScoopIoSocket *socket, int64_t id) {
    scoop_io_request_cancel(socket->loop, id);
}
void scoop_io_udp_receive_cancel(ScoopIoSocket *socket, int64_t id) {
    scoop_io_request_cancel(socket->loop, id);
}
void scoop_io_udp_send_cancel(ScoopIoSocket *socket, int64_t id) {
    scoop_io_request_cancel(socket->loop, id);
}
struct ScoopDatagram scoop_io_udp_take_packet(struct ScoopDatagram *packet) {
    struct ScoopDatagram value = *packet;
    free(packet);
    return value;
}
