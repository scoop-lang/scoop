#if defined(_WIN32)
#include "../windows/socket.h"
typedef ScoopIoSocket TcpEndpoint;
#elif defined(__APPLE__)
#include "../darwin/endpoint.h"
typedef ScoopIoEndpoint TcpEndpoint;
#else
#include "../linux/endpoint.h"
typedef ScoopIoEndpoint TcpEndpoint;
#endif

int scoop_io_tcp_read(TcpEndpoint *endpoint, int64_t id, void *bytes, int64_t count);
int scoop_io_tcp_write(TcpEndpoint *endpoint, int64_t id, void *bytes, int64_t count);

// Owner-only GCLeaf entry: bounded stores, with no allocator, lock or platform call.
bool scoop_io_tcp_enqueue(ScoopIoLoop *loop, void *endpoint, int64_t id, void *bytes, int64_t count,
                          bool writing) {
    ScoopIoTcpSubmissions *submissions = &loop->tcp_submissions;
    if (submissions->count == SCOOP_IO_BATCH_CAPACITY)
        return false;
    submissions->entries[submissions->count++] =
        (ScoopIoTcpSubmission){endpoint, id, bytes, count, writing};
    return true;
}

// All platform work and allocation stays inside this NativeSafe batch boundary.
ScoopIoBatch scoop_io_tcp_flush(ScoopIoLoop *loop) {
    ScoopIoTcpSubmissions *submissions = &loop->tcp_submissions;
    unsigned count = submissions->count;
    submissions->count = 0;
    int64_t completed = 0;
    for (unsigned i = 0; i < count; i++) {
        ScoopIoTcpSubmission entry = submissions->entries[i];
#if defined(__APPLE__) || defined(_WIN32)
        if (scoop_io_tcp_try_transfer(entry.endpoint, entry.id, entry.bytes, entry.count,
                                      entry.writing, &submissions->results[completed])) {
            completed++;
            continue;
        }
#endif
        int status = entry.writing
                         ? scoop_io_tcp_write(entry.endpoint, entry.id, entry.bytes, entry.count)
                         : scoop_io_tcp_read(entry.endpoint, entry.id, entry.bytes, entry.count);
        if (status < 0)
            submissions->results[completed++] =
                (ScoopIoEvent){.id = entry.id, .kind = SCOOP_IO_POSIX_ERROR, .code = -status};
    }
    // Results stay stable until the next flush; completion only queues managed ready work.
    return (ScoopIoBatch){submissions->results, completed};
}
