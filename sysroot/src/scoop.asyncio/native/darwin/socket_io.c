#include "endpoint.h"
#include <errno.h>
#include <limits.h>
#include <sys/uio.h>

typedef enum { TCP_READ, TCP_WRITE, TCP_SHUTDOWN, UDP_READ, UDP_WRITE } SocketAction;
typedef struct {
    ScoopIoWait wait;
    ScoopIoEndpoint *endpoint;
    SocketAction action;
    void *bytes;
    int64_t length;
    struct ScoopSocketAddress address;
    struct ScoopDatagram *packet;
} SocketIo;

static bool writing(SocketAction action) {
    return action == TCP_WRITE || action == TCP_SHUTDOWN || action == UDP_WRITE;
}

static void transfer_ready(void *context);

static void tcp_source_ready(void *context) {
    ScoopIoTcpSource *source = context;
    ScoopIoEndpoint *endpoint = source->endpoint;
    ScoopIoWait *wait = source->writing ? endpoint->output : endpoint->input;
    if (wait)
        transfer_ready(wait);
}

static void tcp_source_retired(void *context) {
    ScoopIoTcpSource *source = context;
    ScoopIoEndpoint *endpoint = source->endpoint;
    ScoopIoLoop *loop = endpoint->loop;
    dispatch_release(source->source);
    source->source = NULL;
    endpoint->active--;
    scoop_io_endpoint_idle(endpoint);
    scoop_io_action_end(loop);
}

static int tcp_wait_fd(ScoopIoEndpoint *endpoint, bool output) {
    ScoopIoTcpSource *source = &endpoint->tcp_sources[output ? 1 : 0];
    if (!source->source) {
        source->source =
            dispatch_source_create(output ? DISPATCH_SOURCE_TYPE_WRITE : DISPATCH_SOURCE_TYPE_READ,
                                   (uintptr_t)endpoint->fd, 0, endpoint->loop->queue);
        if (!source->source)
            return -ENOMEM;
        source->endpoint = endpoint;
        source->writing = output;
        scoop_io_action_begin(endpoint->loop);
        dispatch_set_context(source->source, source);
        dispatch_source_set_event_handler_f(source->source, tcp_source_ready);
        dispatch_source_set_cancel_handler_f(source->source, tcp_source_retired);
        dispatch_activate(source->source);
    } else if (source->suspended) {
        source->suspended = false;
        dispatch_resume(source->source);
    }
    return 0;
}

void scoop_io_tcp_sources_close(ScoopIoEndpoint *endpoint) {
    for (unsigned i = 0; i < 2; i++) {
        ScoopIoTcpSource *source = &endpoint->tcp_sources[i];
        if (!source->source)
            continue;
        endpoint->active++;
        dispatch_source_cancel(source->source);
        if (source->suspended) {
            source->suspended = false;
            dispatch_resume(source->source);
        }
    }
}

static void transfer_finish(ScoopIoWait *wait) {
    SocketIo *request = (void *)wait;
    if (wait->request.completion.event.kind != SCOOP_IO_DATAGRAM)
        free(request->packet);
    if (request->action == TCP_READ || request->action == TCP_WRITE) {
        ScoopIoTcpSource *source =
            &request->endpoint->tcp_sources[writing(request->action) ? 1 : 0];
        if (source->source && !source->suspended) {
            source->suspended = true;
            dispatch_suspend(source->source);
        }
    }
    scoop_io_endpoint_finished(wait, request->endpoint, writing(request->action));
}
static void transfer_cancel(ScoopIoRequest *request) { scoop_io_wait_end((void *)request); }

static ssize_t transfer_datagram(SocketIo *request) {
    struct sockaddr_storage address;
    struct iovec vector = {.iov_base = request->bytes, .iov_len = (size_t)request->length};
    struct msghdr message = {.msg_name = &address, .msg_iov = &vector, .msg_iovlen = 1};
    if (request->action == UDP_WRITE) {
        message.msg_namelen = scoop_socket_address_encode(request->address, &address);
        if (!message.msg_namelen)
            return -1;
        return sendmsg(request->endpoint->fd, &message, 0);
    }
    message.msg_namelen = sizeof(address);
    ssize_t count = recvmsg(request->endpoint->fd, &message, 0);
    if (count >= 0) {
        request->packet->source =
            scoop_socket_address_decode((struct sockaddr *)&address, message.msg_namelen);
        if (!request->packet->source.family)
            return -1;
        request->packet->count = count;
        request->packet->truncated = (message.msg_flags & MSG_TRUNC) != 0;
    }
    return count;
}

// Direction ownership transfers only after the previous native completion is consumed.
bool scoop_io_tcp_try_transfer(ScoopIoEndpoint *endpoint, int64_t id, void *bytes, int64_t length,
                               bool output, ScoopIoEvent *event) {
    *event = (ScoopIoEvent){.id = id};
    if (!output && endpoint->read_error) {
        event->kind = SCOOP_IO_POSIX_ERROR;
        event->code = endpoint->read_error;
        return true;
    }
    if (!output && endpoint->eof) {
        event->kind = SCOOP_IO_EOF;
        return true;
    }
    size_t prefix = (size_t)(length > INT_MAX ? INT_MAX : length);
    ssize_t count;
    do {
        count =
            output ? send(endpoint->fd, bytes, prefix, 0) : recv(endpoint->fd, bytes, prefix, 0);
    } while (count < 0 && errno == EINTR);
    if (count < 0) {
        int error = errno;
        if (error == EAGAIN || error == EWOULDBLOCK)
            return false;
        if (!output)
            endpoint->read_error = error;
        event->kind = SCOOP_IO_POSIX_ERROR;
        event->code = error;
    } else {
        event->count = count;
        if (!output && !count) {
            endpoint->eof = true;
            event->kind = SCOOP_IO_EOF;
        } else
            event->kind = SCOOP_IO_BYTES;
    }
    return true;
}

static void transfer_ready(void *context) {
    SocketIo *request = context;
    if (request->wait.ended)
        return;
    ScoopIoEndpoint *endpoint = request->endpoint;
    ScoopIoEvent *event = &request->wait.request.completion.event;
    if (request->action == TCP_READ || request->action == TCP_WRITE) {
        if (scoop_io_tcp_try_transfer(endpoint, event->id, request->bytes, request->length,
                                      writing(request->action), event)) {
            scoop_io_wait_end(&request->wait);
            return;
        }
        int status = tcp_wait_fd(endpoint, writing(request->action));
        if (status < 0)
            scoop_io_wait_error(&request->wait, -status);
        return;
    } else {
        ssize_t count;
        do {
            count = request->action == TCP_SHUTDOWN ? shutdown(endpoint->fd, SHUT_WR)
                                                    : transfer_datagram(request);
        } while (count < 0 && errno == EINTR);
        if (count >= 0) {
            event->count = count;
            if (request->action == UDP_READ) {
                event->kind = SCOOP_IO_DATAGRAM;
                event->resource = request->packet;
            } else if (request->action == UDP_WRITE)
                event->kind = SCOOP_IO_BYTES;
            scoop_io_wait_end(&request->wait);
            return;
        }
        int error = errno;
        if (error != EAGAIN && error != EWOULDBLOCK) {
            scoop_io_wait_error(&request->wait, error);
            return;
        }
    }
    if (!request->wait.count) {
        int status = scoop_io_wait_fd(&request->wait, endpoint->fd, writing(request->action),
                                      transfer_ready);
        if (status < 0)
            scoop_io_wait_error(&request->wait, -status);
    }
}

static void transfer_start(void *context) {
    SocketIo *request = context;
    ScoopIoLoop *loop = request->wait.request.loop;
    request->endpoint->active++;
    if (writing(request->action))
        request->endpoint->output = &request->wait;
    else
        request->endpoint->input = &request->wait;
    if (!scoop_io_request_start(&request->wait.request))
        scoop_io_wait_end(&request->wait);
    else
        transfer_ready(request);
    scoop_io_action_end(loop);
}

static int submit_transfer(ScoopIoEndpoint *endpoint, int64_t id, SocketAction action, void *bytes,
                           int64_t length, struct ScoopSocketAddress address) {
    SocketIo *request = calloc(1, sizeof(*request));
    if (!request)
        return -ENOMEM;
    if (action == UDP_READ) {
        request->packet = calloc(1, sizeof(*request->packet));
        if (!request->packet) {
            free(request);
            return -ENOMEM;
        }
    }
    request->wait.finish = transfer_finish;
    request->endpoint = endpoint;
    request->action = action;
    request->bytes = bytes;
    request->length = length;
    request->address = address;
    scoop_io_request_submit(&request->wait.request, endpoint->loop, id, transfer_start,
                            transfer_cancel);
    return 0;
}

int scoop_io_tcp_read(ScoopIoEndpoint *endpoint, int64_t id, void *bytes, int64_t count) {
    return submit_transfer(endpoint, id, TCP_READ, bytes, count, (struct ScoopSocketAddress){0});
}
int scoop_io_tcp_write(ScoopIoEndpoint *endpoint, int64_t id, void *bytes, int64_t count) {
    return submit_transfer(endpoint, id, TCP_WRITE, bytes, count, (struct ScoopSocketAddress){0});
}
int scoop_io_tcp_shutdown(ScoopIoEndpoint *endpoint, int64_t id) {
    return submit_transfer(endpoint, id, TCP_SHUTDOWN, NULL, 0, (struct ScoopSocketAddress){0});
}
int scoop_io_udp_receive(ScoopIoEndpoint *endpoint, int64_t id, void *bytes, int64_t count) {
    return submit_transfer(endpoint, id, UDP_READ, bytes, count, (struct ScoopSocketAddress){0});
}
int scoop_io_udp_send(ScoopIoEndpoint *endpoint, int64_t id, void *bytes, int64_t count,
                      struct ScoopSocketAddress address) {
    return submit_transfer(endpoint, id, UDP_WRITE, bytes, count, address);
}
void scoop_io_read_cancel(ScoopIoEndpoint *endpoint, int64_t id) {
    scoop_io_endpoint_cancel(endpoint, id);
}
void scoop_io_tcp_write_cancel(ScoopIoEndpoint *endpoint, int64_t id) {
    scoop_io_endpoint_cancel(endpoint, id);
}
void scoop_io_udp_receive_cancel(ScoopIoEndpoint *endpoint, int64_t id) {
    scoop_io_endpoint_cancel(endpoint, id);
}
void scoop_io_udp_send_cancel(ScoopIoEndpoint *endpoint, int64_t id) {
    scoop_io_endpoint_cancel(endpoint, id);
}
struct ScoopDatagram scoop_io_udp_take_packet(struct ScoopDatagram *packet) {
    struct ScoopDatagram value = *packet;
    free(packet);
    return value;
}
