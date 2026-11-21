#ifndef SCOOP_ASYNCIO_COMMON_PTY_H
#define SCOOP_ASYNCIO_COMMON_PTY_H

#include "../../../scoop.platform/native/pty.h"
#include "work.h"

ScoopIoStatus scoop_io_pty_associate(ScoopIoLoop *loop, ScoopPlatformPty *pty);
ScoopIoStatus scoop_io_pty_watch(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id);
int scoop_io_pty_transfer(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, void *bytes,
                          int64_t count, bool writing);
void scoop_io_pty_cancel(ScoopIoLoop *loop, int64_t id);
ScoopIoStatus scoop_io_pty_close_output(ScoopPlatformPty *pty);
int scoop_io_pty_resize(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id, int32_t columns,
                        int32_t rows);
int scoop_io_pty_end_console(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id);
int scoop_io_pty_close(ScoopIoLoop *loop, ScoopPlatformPty *pty, int64_t id);

static inline ScoopIoStatus scoop_io_pty_status(int error) {
#ifdef _WIN32
    return (ScoopIoStatus){SCOOP_IO_WIN32_ERROR, error};
#else
    return (ScoopIoStatus){SCOOP_IO_POSIX_ERROR, error};
#endif
}

#endif
