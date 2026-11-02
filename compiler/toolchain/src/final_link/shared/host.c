#include <stdio.h>
#include <stdlib.h>

#ifdef _WIN32
#include <windows.h>
static HANDLE ready;
#else
#include <dlfcn.h>
#include <pthread.h>
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t changed = PTHREAD_COND_INITIALIZER;
static int ready;
#endif

typedef int (*Init)(const void *, void *, void *);
static Init initialize;
static int worker_result;

/* The library's identically named helper must bind inside its own image. */
int scoop_shared_helper(void) { return -100; }

static int call_table(void) {
    struct {
        int (*check)(void);
    } api = {0};
    if (initialize(NULL, &api, NULL) || !api.check)
        return 1;
    int first = api.check();
    int second = api.check();
    if (first != 41 || second != 42) {
        fprintf(stderr, "callback values: %d, %d (expected 41, 42)\n", first, second);
        return 1;
    }
    return 0;
}

#ifdef _WIN32
static DWORD WINAPI worker(void *unused) {
    (void)unused;
    if (WaitForSingleObject(ready, INFINITE) != WAIT_OBJECT_0)
        return 1;
    worker_result = call_table();
    return 0;
}
#ifdef SCOOP_TEST_IMPORT
__declspec(dllimport) int scoop_app_init(const void *, void *, void *);
#endif
int wmain(int argc, wchar_t **argv) {
    if (argc != 2)
        return 1;
    ready = CreateEventW(NULL, TRUE, FALSE, NULL);
    HANDLE thread = CreateThread(NULL, 0, worker, NULL, 0, NULL);
    if (!ready || !thread)
        return 2;
    HMODULE module = LoadLibraryW(argv[1]);
    if (!module || GetProcAddress(module, "scoop_shared_helper"))
        return 3;
#ifdef SCOOP_TEST_IMPORT
    initialize = scoop_app_init;
#else
    initialize = (Init)GetProcAddress(module, "scoop_app_init");
#endif
    if (!initialize || call_table())
        return 4;
    if (!SetEvent(ready) || WaitForSingleObject(thread, INFINITE) != WAIT_OBJECT_0)
        return 5;
    DWORD code;
    if (!GetExitCodeThread(thread, &code) || code || worker_result)
        return 6;
    CloseHandle(thread);
    CloseHandle(ready);
#else
static void *worker(void *unused) {
    (void)unused;
    if (pthread_mutex_lock(&lock))
        abort();
    while (!ready) {
        if (pthread_cond_wait(&changed, &lock))
            abort();
    }
    if (pthread_mutex_unlock(&lock))
        abort();
    worker_result = call_table();
    return NULL;
}
int main(int argc, char **argv) {
    if (argc != 2)
        return 1;
    pthread_t thread;
    if (pthread_create(&thread, NULL, worker, NULL))
        return 2;
    void *module = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!module) {
        fprintf(stderr, "%s\n", dlerror());
        return 3;
    }
    if (dlsym(module, "scoop_shared_helper"))
        return 4;
    initialize = (Init)dlsym(module, "scoop_app_init");
    if (!initialize || call_table())
        return 5;
    if (pthread_mutex_lock(&lock))
        return 6;
    ready = 1;
    if (pthread_cond_signal(&changed) || pthread_mutex_unlock(&lock))
        return 7;
    if (pthread_join(thread, NULL) || worker_result)
        return 8;
#endif
    puts("shared library TLS, CRT, unwind and hidden callbacks passed");
    return 0;
}
