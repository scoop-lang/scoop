#include <arpa/inet.h>
#include <inttypes.h>
#include <netinet/tcp.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>

#include <sys/socket.h>

#include "scoop_rt.h"

/* Configured before reactor threads start; publish readiness after all listeners. */
static int expected_shards = 1;
static atomic_int ready_shards;
void echo_configure_shards(int count) { expected_shards = count; }

/* Linux inherits TCP_NODELAY from the listener; the public async API has no
 * socket option. */
void echo_configure_tcp(int port) {
    int found = 0;
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
    if (!found)
        abort();
    if (atomic_fetch_add(&ready_shards, 1) + 1 == expected_shards) {
        fputs("READY\n", stdout);
        fflush(stdout);
    }
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
