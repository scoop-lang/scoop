#define _CRT_SECURE_NO_WARNINGS
#define WIN32_LEAN_AND_MEAN
#include "test_process.h"
#include <io.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

static const wchar_t result_key[] = L"SCOOP_TEST_CASE_RESULT=";

static wchar_t *child_environment(const wchar_t *path) {
    wchar_t *original = GetEnvironmentStringsW();
    if (!original)
        return NULL;
    size_t needed = wcslen(result_key) + wcslen(path) + 2;
    for (const wchar_t *var = original; *var; var += wcslen(var) + 1)
        needed += wcslen(var) + 1;
    wchar_t *block = calloc(needed, sizeof(*block));
    if (block) {
        wchar_t *next = block;
        int inserted = 0;
        for (const wchar_t *var = original; *var; var += wcslen(var) + 1) {
            if (!_wcsnicmp(var, result_key, wcslen(result_key)) ||
                !_wcsnicmp(var, L"SCOOP_TEST_RUN_DIR=", wcslen(L"SCOOP_TEST_RUN_DIR=")))
                continue;
            if (!inserted && _wcsicmp(var, result_key) > 0) {
                wcscpy(next, result_key);
                wcscat(next, path);
                next += wcslen(next) + 1;
                inserted = 1;
            }
            wcscpy(next, var);
            next += wcslen(var) + 1;
        }
        if (!inserted) {
            wcscpy(next, result_key);
            wcscat(next, path);
        }
    }
    FreeEnvironmentStringsW(original);
    return block;
}

static DWORD start_child(const char *key, const wchar_t *path, FILE *output, FILE *error,
                         PROCESS_INFORMATION *process) {
    // CreateProcessW receives the executable and its quoted argv[0] separately.
    wchar_t *image = malloc(32768 * sizeof(wchar_t));
    if (!image)
        return ERROR_NOT_ENOUGH_MEMORY;
    DWORD length = GetModuleFileNameW(NULL, image, 32768);
    DWORD err = length == 0 || length == 32768 ? GetLastError() : 0;
    size_t capacity = (size_t)length + strlen(key) + 24;
    wchar_t *command = calloc(capacity, sizeof(*command));
    wchar_t *env = child_environment(path);
    if (!command || !env)
        err = ERROR_NOT_ENOUGH_MEMORY;
    if (!err) {
        wcscpy(command, L"\"");
        wcscat(command, image);
        wcscat(command, L"\" --scoop-test-child ");
        wchar_t *next = command + wcslen(command);
        while (*key)
            *next++ = (unsigned char)*key++;
    }

    STARTUPINFOEXW startup = {0};
    startup.StartupInfo.cb = sizeof(STARTUPINFOW);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    HANDLE sources[] = {
        GetStdHandle(STD_INPUT_HANDLE),
        output ? (HANDLE)_get_osfhandle(_fileno(output)) : GetStdHandle(STD_OUTPUT_HANDLE),
        error ? (HANDLE)_get_osfhandle(_fileno(error)) : GetStdHandle(STD_ERROR_HANDLE),
    };
    HANDLE *redirects[] = {&startup.StartupInfo.hStdInput, &startup.StartupInfo.hStdOutput,
                           &startup.StartupInfo.hStdError};
    HANDLE handles[3];
    size_t count = 0;
    for (int i = 0; !err && i < 3; ++i) {
        if (!sources[i] || sources[i] == INVALID_HANDLE_VALUE)
            continue;
        if (DuplicateHandle(GetCurrentProcess(), sources[i], GetCurrentProcess(), redirects[i], 0,
                            TRUE, DUPLICATE_SAME_ACCESS))
            handles[count++] = *redirects[i];
        else
            err = GetLastError();
    }
    DWORD flags = CREATE_UNICODE_ENVIRONMENT;
    int initialized = 0;
    if (!err && count) {
        SIZE_T size = 0;
        InitializeProcThreadAttributeList(NULL, 1, 0, &size);
        startup.lpAttributeList = malloc(size);
        if (!startup.lpAttributeList)
            err = ERROR_NOT_ENOUGH_MEMORY;
        else if (!InitializeProcThreadAttributeList(startup.lpAttributeList, 1, 0, &size))
            err = GetLastError();
        else {
            initialized = 1;
            if (!UpdateProcThreadAttribute(startup.lpAttributeList, 0,
                                           PROC_THREAD_ATTRIBUTE_HANDLE_LIST, handles,
                                           count * sizeof(*handles), NULL, NULL))
                err = GetLastError();
        }
        flags |= EXTENDED_STARTUPINFO_PRESENT;
        startup.StartupInfo.cb = sizeof(startup);
    }
    if (!err && !CreateProcessW(image, command, NULL, NULL, count != 0, flags, env, NULL,
                                &startup.StartupInfo, process))
        err = GetLastError();
    if (initialized)
        DeleteProcThreadAttributeList(startup.lpAttributeList);
    free(startup.lpAttributeList);
    for (size_t i = 0; i < count; ++i)
        CloseHandle(handles[i]);
    free(env);
    free(command);
    free(image);
    return err;
}

int scoop_test_spawn(const char *key, const wchar_t *path, FILE *output, FILE *error,
                     struct TestExit *status, char *reason, size_t capacity) {
    PROCESS_INFORMATION process = {0};
    DWORD err = start_child(key, path, output, error, &process);
    if (err) {
        snprintf(reason, capacity, "cannot start test child (Windows error %lu)",
                 (unsigned long)err);
        return 1;
    }
    CloseHandle(process.hThread);
    DWORD code;
    if (WaitForSingleObject(process.hProcess, INFINITE) != WAIT_OBJECT_0 ||
        !GetExitCodeProcess(process.hProcess, &code)) {
        err = GetLastError();
        CloseHandle(process.hProcess);
        snprintf(reason, capacity, "cannot wait for test child (Windows error %lu)",
                 (unsigned long)err);
        return -1;
    }
    CloseHandle(process.hProcess);
    *status = (struct TestExit){1, (int32_t)code, 0};
    return 0;
}
