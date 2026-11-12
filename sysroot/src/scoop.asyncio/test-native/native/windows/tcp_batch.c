#include "socket_test.h"

static int readable(SOCKET socket) {
    fd_set input;
    FD_ZERO(&input);
    FD_SET(socket, &input);
    struct timeval timeout = {.tv_sec = 5};
    return select(0, &input, NULL, NULL, &timeout) == 1 ? 0 : -1;
}

static int ready_transfer(ScoopIoSocket *socket, int64_t id, void *bytes, int64_t length,
                          bool output, int kind) {
    bool inline_result =
        output || socket->skip_read_packet || socket->eof || socket->read_error.code;
    CHECK(scoop_io_tcp_enqueue(socket->loop, socket, id, bytes, length, output));
    ScoopIoBatch batch = scoop_io_tcp_flush(socket->loop);
    ScoopIoEvent event;
    if (inline_result) {
        CHECK(batch.count == 1);
        event = batch.records[0];
    } else {
        CHECK(batch.count == 0 && socket->active == 1 && socket->loop->requests);
        CHECK(!test_result(socket->loop, &event));
    }
    CHECK(event.id == id && event.kind == kind);
    CHECK(event.count == (kind == SCOOP_IO_EOF ? 0 : length));
    CHECK(socket->active == 0 && !socket->loop->requests);
    CHECK(test_completion_count(socket->loop) == 0);
    return 0;
}

static int default_packet_mode(bool ipv6) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    int family = ipv6 ? 6 : 4;
    ScoopIoSocket *listener = scoop_io_socket_new(loop, family);
    ScoopIoSocket *client = scoop_io_socket_new(loop, family);
    ScoopIoSocket *server = scoop_io_socket_new(loop, family);
    CHECK(listener && client && server);
    CHECK(!scoop_io_socket_create(listener).code && !scoop_io_socket_create(client).code);
    struct sockaddr_storage address;
    int length = scoop_socket_address_encode(test_loopback(ipv6), &address);
    CHECK(length && !bind(listener->socket, (struct sockaddr *)&address, length));
    CHECK(!listen(listener->socket, 1));
    CHECK(!getsockname(listener->socket, (struct sockaddr *)&address, &length));
    CHECK(!connect(client->socket, (struct sockaddr *)&address, length));
    server->socket = accept(listener->socket, NULL, NULL);
    CHECK(server->socket != INVALID_SOCKET);
    u_long nonblocking = 1;
    CHECK(!ioctlsocket(client->socket, FIONBIO, &nonblocking));
    CHECK(!ioctlsocket(server->socket, FIONBIO, &nonblocking));
    CHECK(!scoop_io_socket_associate(client).code && !scoop_io_socket_associate(server).code);
    // These real sockets have never enabled packet suppression; synchronous reads must wait.
    CHECK(!client->skip_read_packet && !server->skip_read_packet);
    unsigned char input[3] = {0}, output[3] = {0x80, 0xff, 0x01};
    CHECK(send(client->socket, (const char *)output, sizeof(output), 0) == (int)sizeof(output));
    CHECK(!readable(server->socket));
    CHECK(!ready_transfer(server, 1, input, sizeof(input), false, SCOOP_IO_BYTES));
    CHECK(!memcmp(input, output, sizeof(input)));

    CHECK(scoop_io_tcp_enqueue(loop, server, 2, input, sizeof(input), false));
    CHECK(scoop_io_tcp_flush(loop).count == 0 && server->active == 1);
    scoop_io_read_cancel(server, 1);
    CHECK(send(client->socket, (const char *)output, sizeof(output), 0) == (int)sizeof(output));
    ScoopIoEvent event;
    CHECK(!test_result(loop, &event) && event.id == 2 && event.kind == SCOOP_IO_BYTES &&
          event.count == sizeof(input));
    CHECK(!memcmp(input, output, sizeof(input)));
    CHECK(scoop_io_tcp_enqueue(loop, server, 3, input, sizeof(input), false));
    CHECK(scoop_io_tcp_flush(loop).count == 0 && server->active == 1);
    scoop_io_read_cancel(server, 3);
    CHECK(server->active == 1);
    CHECK(!test_result(loop, &event) && event.id == 3 && event.kind == SCOOP_IO_WINSOCK_ERROR &&
          event.code == WSA_OPERATION_ABORTED);
    CHECK(!shutdown(client->socket, SD_SEND) && !readable(server->socket));
    CHECK(!ready_transfer(server, 4, input, 1, false, SCOOP_IO_EOF));
    CHECK(!ready_transfer(server, 5, input, 1, false, SCOOP_IO_EOF));
    CHECK(!test_closed(client, 6) && !test_closed(server, 7) && !test_closed(listener, 8));
    CHECK(!test_drain(loop));
    return 0;
}

int scoop_test_iocp_tcp_batch(bool ipv6) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    ScoopIoSocket *listener = NULL, *client = NULL, *server = NULL;
    CHECK(!test_pair(loop, ipv6, &listener, &client, &server));
    unsigned char input[3] = {0}, output[3] = {0x80, 0xff, 0x01};

    // An empty receive keeps its real overlapped request until the cancellation packet.
    CHECK(scoop_io_tcp_enqueue(loop, server, 5, input, sizeof(input), false));
    CHECK(scoop_io_tcp_flush(loop).count == 0);
    CHECK(server->active == 1 && loop->requests);
    scoop_io_read_cancel(server, 5);
    CHECK(server->active == 1 && loop->requests);
    ScoopIoEvent event;
    CHECK(!test_result(loop, &event) && event.id == 5 && event.kind == SCOOP_IO_WINSOCK_ERROR &&
          event.code == WSA_OPERATION_ABORTED);
    CHECK(!server->read_error.code);

    // Ready transfers use the native batch when the socket supports packet suppression.
    for (unsigned i = 0; i < 8; i++) {
        output[2] = (unsigned char)i;
        CHECK(!ready_transfer(client, 6, output, sizeof(output), true, SCOOP_IO_BYTES));
        CHECK(!readable(server->socket));
        CHECK(!ready_transfer(server, 7, input, 2, false, SCOOP_IO_BYTES));
        CHECK(!ready_transfer(server, 8, input + 2, 1, false, SCOOP_IO_BYTES));
        CHECK(memcmp(input, output, sizeof(input)) == 0);
    }

    CHECK(scoop_io_tcp_enqueue(loop, server, 17, input, sizeof(input), false));
    CHECK(scoop_io_tcp_flush(loop).count == 0 && server->active == 1);
    scoop_io_read_cancel(server, 8);
    CHECK(send(client->socket, (const char *)output, sizeof(output), 0) == (int)sizeof(output));
    CHECK(!test_result(loop, &event) && event.id == 17 && event.kind == SCOOP_IO_BYTES &&
          event.count == sizeof(input));
    CHECK(!memcmp(input, output, sizeof(input)));

    CHECK(!scoop_io_tcp_shutdown(client, 9));
    CHECK(!test_result(loop, &event) && event.id == 9 && event.kind == SCOOP_IO_DONE);
    CHECK(!readable(server->socket));
    CHECK(!ready_transfer(server, 10, input, 1, false, SCOOP_IO_EOF));
    CHECK(!ready_transfer(server, 11, input, 1, false, SCOOP_IO_EOF));
    CHECK(!ready_transfer(server, 12, output, sizeof(output), true, SCOOP_IO_BYTES));
    CHECK(!readable(client->socket));
    CHECK(!ready_transfer(client, 13, input, sizeof(input), false, SCOOP_IO_BYTES));
    CHECK(memcmp(input, output, sizeof(input)) == 0);
    CHECK(!test_closed(client, 14) && !test_closed(server, 15) && !test_closed(listener, 16));
    CHECK(!test_drain(loop));
    CHECK(!default_packet_mode(ipv6));
    return 0;
}

int scoop_test_iocp_tcp_write_wait(bool ipv6) {
    ScoopIoCreated created = scoop_io_loop_create();
    CHECK(!created.status.code);
    ScoopIoLoop *loop = created.pointer;
    ScoopIoSocket *listener = NULL, *client = NULL, *server = NULL;
    CHECK(!test_pair(loop, ipv6, &listener, &client, &server));
    int capacity = 4096;
    CHECK(setsockopt(client->socket, SOL_SOCKET, SO_SNDBUF, (const char *)&capacity,
                     sizeof(capacity)) == 0);
    CHECK(setsockopt(server->socket, SOL_SOCKET, SO_RCVBUF, (const char *)&capacity,
                     sizeof(capacity)) == 0);
    const unsigned length = 65536;
    unsigned char *output = malloc(length), *input = malloc(length);
    CHECK(output && input);
    memset(output, 0x31, length);
    for (unsigned round = 0; round < 2; round++) {
        int64_t accepted = 0;
        ScoopIoEvent event;
        for (unsigned i = 0; i < 64; i++) {
            CHECK(!scoop_io_tcp_write(client, 5, output, length));
            if (client->active)
                break;
            CHECK(!test_result(loop, &event) && event.id == 5 && event.kind == SCOOP_IO_BYTES &&
                  event.count > 0 && event.count <= length);
            accepted += event.count;
        }
        CHECK(client->active == 1 && client->output);
        bool written = false;
        int64_t received = 0;
        while (!written || received < accepted) {
            int64_t count = written && accepted - received < length ? accepted - received : length;
            CHECK(!scoop_io_tcp_read(server, 6, input, count));
            bool read = false;
            while (!read) {
                CHECK(!test_result(loop, &event) && event.kind == SCOOP_IO_BYTES);
                CHECK(event.count > 0 && event.count <= length);
                if (event.id == 5) {
                    CHECK(!written);
                    written = true;
                    accepted += event.count;
                } else {
                    CHECK(event.id == 6);
                    read = true;
                    received += event.count;
                    for (int64_t i = 0; i < event.count; i++)
                        CHECK(input[i] == 0x31);
                }
            }
        }
        CHECK(written && received == accepted && !client->active && !server->active);
    }
    free(input);
    free(output);
    CHECK(!test_closed(client, 7) && !test_closed(server, 8) && !test_closed(listener, 9));
    CHECK(!test_drain(loop));
    return 0;
}
