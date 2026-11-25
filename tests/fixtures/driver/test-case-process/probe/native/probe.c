#ifdef _WIN32
#define _CRT_SECURE_NO_WARNINGS
#include <process.h>
#include <windows.h>
#define current_pid _getpid
#else
#include <signal.h>
#include <unistd.h>
#define current_pid getpid
#endif
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

void process_fixture_trace(int32_t event) {
#ifdef _WIN32
    const wchar_t *path = _wgetenv(L"SCOOP_FIXTURE_TRACE");
    FILE *file = path ? _wfopen(path, L"ab") : NULL;
#else
    const char *path = getenv("SCOOP_FIXTURE_TRACE");
    FILE *file = path ? fopen(path, "ab") : NULL;
#endif
    if (file) {
        fprintf(file, "%ld %d\n", (long)current_pid(), (int)event);
        fclose(file);
    }
}

static int mode;
static void shutdown_output(void) {
    fputs("shutdown\n", stderr);
    fflush(stderr);
    if (mode == 5)
        _Exit(19);
}

int32_t process_fixture_body(void) {
    const char *setting = getenv("SCOOP_FIXTURE_MODE");
    mode = setting ? atoi(setting) : 0;
    if (!mode)
        return 0;
    if (atexit(shutdown_output))
        abort();
    fputs("{\"event\":\"complete\"}\n", stdout);
    fflush(stdout);
    switch (mode) {
    case 1:
        exit(0);
    case 2:
        _Exit(23);
    case 3:
#ifdef _WIN32
        ExitProcess(0xc0000005u);
#else
        raise(SIGTERM);
#endif
        break;
    case 4:
        for (int i = 0; i < 131072; ++i) {
            fputc(0xff, stdout);
            fputc('x', stderr);
        }
        break;
    }
    return mode;
}
