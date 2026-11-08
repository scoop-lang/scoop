#define _GNU_SOURCE 1
#include "environment.h"
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <wchar.h>

ScoopPlatformString scoop_platform_executable_path_result(void) {
    DWORD capacity = 256;
    WCHAR *buffer = NULL;
    for (;;) {
        WCHAR *resized = realloc(buffer, (size_t)capacity * sizeof(WCHAR));
        if (resized == NULL) {
            free(buffer);
            return (ScoopPlatformString){NULL, 0, ERROR_NOT_ENOUGH_MEMORY};
        }
        buffer = resized;
        DWORD count = GetModuleFileNameW(NULL, buffer, capacity);
        if (count == 0) {
            DWORD error = GetLastError();
            free(buffer);
            return (ScoopPlatformString){NULL, 0, (int32_t)error};
        }
        if (count < capacity)
            return (ScoopPlatformString){buffer, (int64_t)count, 0};
        if (capacity > MAXDWORD / 2) {
            free(buffer);
            return (ScoopPlatformString){NULL, 0, ERROR_BUFFER_OVERFLOW};
        }
        capacity *= 2;
    }
}
#else
#include <errno.h>
#include <unistd.h>
#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif

ScoopPlatformString scoop_platform_executable_path_result(void) {
#ifdef __APPLE__
    uint32_t capacity = 0;
    (void)_NSGetExecutablePath(NULL, &capacity);
    char *buffer = malloc(capacity);
    if (buffer == NULL)
        return (ScoopPlatformString){NULL, 0, ENOMEM};
    if (_NSGetExecutablePath(buffer, &capacity) != 0) {
        free(buffer);
        return (ScoopPlatformString){NULL, 0, ERANGE};
    }
    char *path = realpath(buffer, NULL);
    int error = path == NULL ? errno : 0;
    free(buffer);
    return (ScoopPlatformString){path, path == NULL ? 0 : (int64_t)strlen(path), error};
#else
    size_t capacity = 256;
    char *buffer = NULL;
    for (;;) {
        char *resized = realloc(buffer, capacity);
        if (resized == NULL) {
            free(buffer);
            return (ScoopPlatformString){NULL, 0, ENOMEM};
        }
        buffer = resized;
        ssize_t count = readlink("/proc/self/exe", buffer, capacity);
        if (count < 0) {
            int error = errno;
            free(buffer);
            return (ScoopPlatformString){NULL, 0, error};
        }
        if ((size_t)count < capacity)
            return (ScoopPlatformString){buffer, count, 0};
        if (capacity > SIZE_MAX / 2) {
            free(buffer);
            return (ScoopPlatformString){NULL, 0, EOVERFLOW};
        }
        capacity *= 2;
    }
#endif
}
#endif
