#ifndef SCOOP_ASYNCIO_PTY_UNIX_H
#define SCOOP_ASYNCIO_PTY_UNIX_H

#include "../loop.h"
#include <errno.h>
#include <unistd.h>

static inline bool scoop_io_pty_try_transfer(ScoopIoEvent *event, int fd, void *bytes,
                                              int64_t count, bool writing) {
    ssize_t result;
    do {
        result = writing ? write(fd, bytes, (size_t)count) : read(fd, bytes, (size_t)count);
    } while (result < 0 && errno == EINTR);
#ifdef __linux__
    if (result < 0 && !writing && errno == EIO)
        result = 0; // The slave has closed and the master buffer is empty.
#endif
    if (result < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK)
            return false;
        event->kind = SCOOP_IO_POSIX_ERROR;
        event->code = errno;
    } else {
        event->kind = !writing && result == 0 ? SCOOP_IO_EOF : SCOOP_IO_BYTES;
        event->count = result;
    }
    return true;
}

#endif
