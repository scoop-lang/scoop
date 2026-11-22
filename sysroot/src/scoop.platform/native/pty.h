#ifndef SCOOP_PLATFORM_PTY_H
#define SCOOP_PLATFORM_PTY_H

#include "process_native.h"

#ifdef _WIN32
typedef HRESULT(WINAPI *ScoopPtyResize)(HPCON, COORD);
typedef void(WINAPI *ScoopPtyClose)(HPCON);
#endif

typedef struct ScoopPlatformPty {
    ScoopPlatformProcess process;
    ScoopPlatformFileHandle input, output;
#ifdef _WIN32
    HPCON console;
    ScoopPtyResize resize_console;
    ScoopPtyClose close_console;
#endif
} ScoopPlatformPty;

typedef struct {
    ScoopPlatformPty *pty;
    int32_t error;
} ScoopPlatformPtyOpened;

#ifdef _WIN32
typedef WCHAR ScoopPtyChar;
int32_t scoop_platform_pty_close_output(ScoopPlatformPty *pty);
void scoop_platform_pty_close_console(ScoopPlatformPty *pty);
#else
typedef char ScoopPtyChar;
#endif

ScoopPlatformPtyOpened scoop_platform_pty_spawn(
    const ScoopPtyChar *executable, ScoopPtyChar *arguments, int64_t argument_count,
    ScoopPtyChar *environment, int64_t environment_count, const ScoopPtyChar *directory,
    int32_t columns, int32_t rows);
int32_t scoop_platform_pty_resize(ScoopPlatformPty *pty, int32_t columns, int32_t rows);
ScoopPlatformProcessWait scoop_platform_pty_try_wait(ScoopPlatformPty *pty);
/* Final cleanup, called after requests have retired. May terminate and wait for the child. */
int32_t scoop_platform_pty_close(ScoopPlatformPty *pty);

#endif
