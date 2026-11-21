#include "pty.h"
#include "process_windows.h"
#include <stdlib.h>
#include <wchar.h>

typedef HRESULT(WINAPI *CreateConsole)(COORD, HANDLE, HANDLE, DWORD, HPCON *);

static DWORD console_error(HRESULT status) {
    if (SUCCEEDED(status))
        return 0;
    return HRESULT_FACILITY(status) == FACILITY_WIN32 ? HRESULT_CODE(status) : ERROR_GEN_FAILURE;
}

/* The server belongs to the host IOCP and the client is a synchronous ConPTY endpoint. */
static DWORD open_pipe(bool host_reads, ScoopPlatformFileHandle *host, HANDLE *client) {
    static volatile LONG64 next_pipe;
    WCHAR name[96];
    if (swprintf(name, sizeof(name) / sizeof(*name), L"\\\\.\\pipe\\scoop-pty-%lu-%lld",
                 (unsigned long)GetCurrentProcessId(), (long long)InterlockedIncrement64(&next_pipe)) <
        0)
        return ERROR_INVALID_NAME;
    DWORD access = host_reads ? PIPE_ACCESS_INBOUND : PIPE_ACCESS_OUTBOUND;
    HANDLE server =
        CreateNamedPipeW(name, access | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
                         PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                         1, 0, 0, 0, NULL);
    if (server == INVALID_HANDLE_VALUE)
        return GetLastError();
    *host = scoop_file_wrap(server);
    *client = CreateFileW(name, host_reads ? GENERIC_WRITE : GENERIC_READ, 0, NULL, OPEN_EXISTING,
                          FILE_ATTRIBUTE_NORMAL, NULL);
    if (*client == INVALID_HANDLE_VALUE)
        return GetLastError();
    return 0;
}

static DWORD create_console(ScoopPlatformPty *pty, int32_t columns, int32_t rows) {
    HMODULE kernel = GetModuleHandleW(L"kernel32.dll");
    if (!kernel)
        return GetLastError();
    CreateConsole create = (CreateConsole)GetProcAddress(kernel, "CreatePseudoConsole");
    pty->resize_console = (ScoopPtyResize)GetProcAddress(kernel, "ResizePseudoConsole");
    pty->close_console = (ScoopPtyClose)GetProcAddress(kernel, "ClosePseudoConsole");
    if (!create || !pty->resize_console || !pty->close_console)
        return ERROR_NOT_SUPPORTED;
    HANDLE input = INVALID_HANDLE_VALUE;
    HANDLE output = INVALID_HANDLE_VALUE;
    DWORD error = open_pipe(false, &pty->input, &input);
    if (!error)
        error = open_pipe(true, &pty->output, &output);
    if (!error)
        error = console_error(
            create((COORD){(SHORT)columns, (SHORT)rows}, input, output, 0, &pty->console));
    if (output != INVALID_HANDLE_VALUE)
        (void)CloseHandle(output);
    if (input != INVALID_HANDLE_VALUE)
        (void)CloseHandle(input);
    return error;
}

static DWORD launch_child(ScoopPlatformPty *pty, const WCHAR *executable, WCHAR *command,
                          WCHAR *environment, const WCHAR *directory) {
    SIZE_T size = 0;
    (void)InitializeProcThreadAttributeList(NULL, 1, 0, &size);
    if (!size)
        return GetLastError();
    STARTUPINFOEXW startup = {0};
    startup.StartupInfo.cb = sizeof(startup);
    startup.lpAttributeList = malloc(size);
    if (!startup.lpAttributeList)
        return ERROR_NOT_ENOUGH_MEMORY;
    DWORD error = 0;
    if (!InitializeProcThreadAttributeList(startup.lpAttributeList, 1, 0, &size)) {
        error = GetLastError();
    } else {
        PROCESS_INFORMATION process = {0};
        if (!UpdateProcThreadAttribute(startup.lpAttributeList, 0,
                                       PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, pty->console,
                                       sizeof(pty->console), NULL, NULL) ||
            !CreateProcessW(executable, command, NULL, NULL, FALSE,
                            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT, environment,
                            directory, &startup.StartupInfo, &process)) {
            error = GetLastError();
        } else {
            (void)CloseHandle(process.hThread);
            pty->process.value = (uint64_t)(uintptr_t)process.hProcess;
        }
        DeleteProcThreadAttributeList(startup.lpAttributeList);
    }
    free(startup.lpAttributeList);
    return error;
}

ScoopPlatformPtyOpened scoop_platform_pty_spawn(
    const WCHAR *executable, WCHAR *arguments, int64_t argument_count, WCHAR *environment,
    int64_t environment_count, const WCHAR *directory, int32_t columns, int32_t rows) {
    ScoopPlatformPty *pty = calloc(1, sizeof(*pty));
    if (!pty)
        return (ScoopPlatformPtyOpened){NULL, ERROR_NOT_ENOUGH_MEMORY};
    pty->process.value = pty->input.value = pty->output.value = UINT64_MAX;
    DWORD error = 0;
    WCHAR *command = scoop_process_command_line(arguments, argument_count, &error);
    WCHAR *block = command ? scoop_process_environment(environment, environment_count, &error)
                            : NULL;
    WCHAR *application = block ? scoop_process_absolute_path(executable, &error) : NULL;
    WCHAR *cwd = application ? scoop_process_absolute_path(directory, &error) : NULL;
    if (!error)
        error = create_console(pty, columns, rows);
    if (!error)
        error = launch_child(pty, application, command, block, cwd);
    free(cwd);
    free(application);
    free(block);
    free(command);
    if (!error)
        return (ScoopPlatformPtyOpened){pty, 0};
    (void)scoop_platform_pty_close(pty);
    return (ScoopPlatformPtyOpened){NULL, (int32_t)error};
}

int32_t scoop_platform_pty_resize(ScoopPlatformPty *pty, int32_t columns, int32_t rows) {
    return (int32_t)console_error(
        pty->resize_console(pty->console, (COORD){(SHORT)columns, (SHORT)rows}));
}
