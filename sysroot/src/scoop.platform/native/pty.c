#include "pty.h"
#include <stdlib.h>

ScoopPlatformProcessWait scoop_platform_pty_try_wait(ScoopPlatformPty *pty) {
    ScoopPlatformProcessWait result = scoop_platform_process_wait(pty->process, true);
    if (result.status >= 0 || result.status == -3)
        pty->process.value = UINT64_MAX;
    return result;
}

static int32_t close_handle(ScoopPlatformFileHandle handle) {
    return handle.value == UINT64_MAX ? 0 : scoop_platform_handle_close(handle).error;
}

int32_t scoop_platform_pty_close(ScoopPlatformPty *pty) {
    bool owns_child = pty->process.value != UINT64_MAX;
    int32_t termination = owns_child ? scoop_platform_process_signal(pty->process, true) : 0;
    int32_t error = termination;
#ifdef _WIN32
    int32_t output_error = scoop_platform_pty_close_output(pty);
    scoop_platform_pty_close_console(pty);
    int32_t input_error = close_handle(pty->input);
    if (!error)
        error = output_error ? output_error : input_error;
#else
    int32_t master_error = close_handle(pty->output);
    if (!error)
        error = master_error;
#endif
    if (owns_child) {
        ScoopPlatformProcessWait state =
            scoop_platform_process_wait(pty->process, termination != 0);
        if (state.status == -1 || state.status == -2)
            scoop_platform_process_abandon(pty->process);
        if (!error)
            error = state.error;
    }
    free(pty);
    return error;
}

#ifdef _WIN32
int32_t scoop_platform_pty_close_output(ScoopPlatformPty *pty) {
    int32_t error = close_handle(pty->output);
    pty->output.value = UINT64_MAX;
    return error;
}

void scoop_platform_pty_close_console(ScoopPlatformPty *pty) {
    if (pty->console) {
        pty->close_console(pty->console);
        pty->console = NULL;
    }
}
#endif
