#include <inttypes.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>

#if defined(_WIN32)
#include <winsock2.h>

#include <windows.h>

#include <processsnapshot.h>
#else
#include <arpa/inet.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#endif

#include "scoop_rt.h"

/* Configured before reactor threads start; publish readiness after all listeners. */
static int expected_shards = 1;
static atomic_int ready_shards;
void echo_configure_shards(int count) { expected_shards = count; }

/* The public async API has no socket option; this runs only during connection setup. */
static void configure_tcp_sockets(int port) {
    int found = 0;
#if defined(_WIN32)
    /* SOCKET values are handles, not small CRT descriptors. Snapshot only this process. */
    HANDLE process = GetCurrentProcess();
    HPSS snapshot;
    HPSSWALK marker;
    if (PssCaptureSnapshot(process, PSS_CAPTURE_HANDLES, 0, &snapshot) != ERROR_SUCCESS)
        abort();
    if (PssWalkMarkerCreate(NULL, &marker) != ERROR_SUCCESS)
        abort();
    PSS_HANDLE_ENTRY entry;
    DWORD status;
    while ((status = PssWalkSnapshot(snapshot, PSS_WALK_HANDLES, marker, &entry, sizeof(entry))) ==
           ERROR_SUCCESS) {
        SOCKET socket = (SOCKET)entry.Handle;
        struct sockaddr_in address;
        int size = sizeof(address);
        if (getsockname(socket, (struct sockaddr *)&address, &size) == 0 &&
            address.sin_family == AF_INET && ntohs(address.sin_port) == port) {
            int enabled = 1;
            if (setsockopt(socket, IPPROTO_TCP, TCP_NODELAY, (const char *)&enabled,
                           sizeof(enabled)) != 0)
                abort();
            found = 1;
        }
    }
    if (status != ERROR_NO_MORE_ITEMS || PssWalkMarkerFree(marker) != ERROR_SUCCESS ||
        PssFreeSnapshot(process, snapshot) != ERROR_SUCCESS)
        abort();
#else
    for (int fd = 0; fd < 1024; fd++) {
        struct sockaddr_in address;
        socklen_t size = sizeof(address);
        if (getsockname(fd, (struct sockaddr *)&address, &size) == 0 &&
            address.sin_family == AF_INET && ntohs(address.sin_port) == port) {
            int enabled = 1;
            if (setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &enabled, sizeof(enabled)) != 0)
                abort();
            found = 1;
        }
    }
#endif
    if (!found)
        abort();
}

void echo_configure_tcp(int port) {
    configure_tcp_sockets(port);
    if (atomic_fetch_add(&ready_shards, 1) + 1 == expected_shards) {
        fputs("READY\n", stdout);
        fflush(stdout);
    }
}

void echo_configure_accepted_tcp(int port) {
#if defined(__APPLE__) || defined(_WIN32)
    /* Set accepted sockets explicitly where listener inheritance is not assumed. */
    configure_tcp_sockets(port);
#else
    (void)port;
#endif
}

void echo_metrics(void) {
    ScoopGcMetrics metrics;
    scoop_rt_gc_debug_metrics(&metrics);
    printf("METRICS {\"allocated_bytes\":%" PRIu64 ",\"minor_collections\":%" PRIu64
           ",\"full_collections\":%" PRIu64 ",\"pause_ns\":%" PRIu64 ",\"stop_wait_ns\":%" PRIu64
           "}\n",
           metrics.allocated_bytes, metrics.minor_collections, metrics.full_collections,
           metrics.pause_ns, metrics.stop_wait_ns);
    fflush(stdout);
}
