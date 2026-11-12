#include "process_native.h"

typedef struct {
    void *handle;
    int32_t error;
} ScoopPlatformPipeThread;

ScoopPlatformPipeThread scoop_platform_process_pipe_thread(void) {
    HANDLE handle = NULL;
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &handle,
                         THREAD_TERMINATE, FALSE, 0))
        return (ScoopPlatformPipeThread){NULL, (int32_t)GetLastError()};
    return (ScoopPlatformPipeThread){handle, 0};
}

int32_t scoop_platform_process_pipe_cancel(void *thread) {
    if (CancelSynchronousIo(thread))
        return 0;
    DWORD error = GetLastError();
    return error == ERROR_NOT_FOUND ? 0 : (int32_t)error;
}

int32_t scoop_platform_process_pipe_thread_close(void *thread) {
    return CloseHandle(thread) ? 0 : (int32_t)GetLastError();
}
