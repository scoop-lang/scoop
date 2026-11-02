#include "scoop_app.h"
#include "threading.h"

#include <assert.h>
#include <fenv.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef _WIN32
#include <dlfcn.h>
#else
#include <dbghelp.h>
#include <fcntl.h>
#include <io.h>
#endif

typedef int32_t (*InitFn)(const void *, void *, ScoopAppError *);
typedef struct {
  int32_t mode;
  int32_t (*service)(int32_t);
} HostApi;
typedef struct {
  int32_t (*run)(int32_t, void *);
  void *context;
  void (*close)(void *);
  void *close_context;
} AppApi;

static InitFn initialize;
static HostApi host;
static AppApi app;
static ScoopAppError first_error;
static int32_t first_status;
static M13Mutex mutex = M13_MUTEX_INITIALIZER;
static M13Condition condition = M13_CONDITION_INITIALIZER;
static int loaded, entered, proceed, returned, finished;
static int blocked, collected;

static void empty_error(ScoopAppError error) {
  assert(error.bytes == NULL && error.length == 0);
}

static ScoopAppError dirty_error(void) {
  return (ScoopAppError){(const uint8_t *)"old", 3};
}

static void set_host_environment(void) {
  assert(fesetround(FE_UPWARD) == 0);
  assert(feclearexcept(FE_ALL_EXCEPT) == 0);
  assert(feraiseexcept(FE_INVALID) == 0);
}

static void check_host_environment(void) {
  assert(fegetround() == FE_UPWARD);
  assert(fetestexcept(FE_ALL_EXCEPT) == FE_INVALID);
}

static int32_t service(int32_t value) {
  if (value == 3) {
    assert(m13_mutex_lock(&mutex) == 0);
    blocked = 1;
    assert(m13_condition_broadcast(&condition) == 0);
    while (!collected) {
      assert(m13_condition_wait(&condition, &mutex) == 0);
    }
    assert(m13_mutex_unlock(&mutex) == 0);
    return 30;
  }
  if (value == 1) {
    set_host_environment();
    int32_t result = app.run(2, app.context);
    check_host_environment();
    return result;
  }
  assert(value == 10);
  ScoopAppError error = dirty_error();
  AppApi untouched = {0};
  assert(initialize(&host, &untouched, &error) == SCOOP_APP_INIT_IN_PROGRESS);
  empty_error(error);
  assert(untouched.run == NULL && untouched.context == NULL);
  assert(m13_mutex_lock(&mutex) == 0);
  entered = 1;
  assert(m13_condition_broadcast(&condition) == 0);
  while (!proceed) {
    assert(m13_condition_wait(&condition, &mutex) == 0);
  }
  assert(m13_mutex_unlock(&mutex) == 0);
  assert(fesetround(FE_DOWNWARD) == 0);
  return 24;
}

static M13ThreadResult M13_THREAD_CALL initialize_thread(void *unused) {
  (void)unused;
  assert(m13_mutex_lock(&mutex) == 0);
  while (!loaded) {
    assert(m13_condition_wait(&condition, &mutex) == 0);
  }
  assert(m13_mutex_unlock(&mutex) == 0);
  set_host_environment();
  first_error = dirty_error();
  first_status = initialize(&host, &app, &first_error);
  check_host_environment();
  assert(m13_mutex_lock(&mutex) == 0);
  returned = 1;
  assert(m13_condition_broadcast(&condition) == 0);
  assert(m13_mutex_unlock(&mutex) == 0);
  return 0;
}

static void run_callbacks(void) {
  set_host_environment();
  assert(app.run(5, app.context) == 29);
  check_host_environment();
  assert(app.run(1, app.context) == 27);
  check_host_environment();
}

static M13ThreadResult M13_THREAD_CALL callback_thread(void *unused) {
  (void)unused;
  assert(m13_mutex_lock(&mutex) == 0);
  while (!finished) {
    assert(m13_condition_wait(&condition, &mutex) == 0);
  }
  assert(m13_mutex_unlock(&mutex) == 0);
  if (host.mode == 0) {
    run_callbacks();
    set_host_environment();
    assert(app.run(3, app.context) == 31);
    check_host_environment();
  }
  return 0;
}

static void load_app(const char *path) {
#ifdef _WIN32
  HMODULE module = LoadLibraryA(path);
  assert(module != NULL);
  FARPROC symbol = GetProcAddress(module, "scoop_app_init");
  assert(GetProcAddress(module, "scoop_rt_init_native_app") == NULL);
#else
  void *module = dlopen(path, RTLD_NOW | RTLD_LOCAL);
  if (module == NULL) {
    fprintf(stderr, "%s\n", dlerror());
    abort();
  }
  void *symbol = dlsym(module, "scoop_app_init");
  assert(dlsym(module, "scoop_rt_init_native_app") == NULL);
#endif
  assert(symbol != NULL);
  _Static_assert(sizeof(symbol) == sizeof(initialize), "function pointer size");
  memcpy(&initialize, &symbol, sizeof(initialize));
  /* The module remains loaded for the process lifetime. */
}

int main(int argc, char **argv) {
  assert(argc == 3 || argc == 4);
  int want_trace = argc == 4 && strcmp(argv[3], "debug") == 0;
#ifdef _WIN32
  assert(_setmode(_fileno(stdout), _O_BINARY) != -1);
  HANDLE symbols_process;
  HANDLE process = GetCurrentProcess();
  assert(DuplicateHandle(process, process, process, &symbols_process, 0, FALSE,
                         DUPLICATE_SAME_ACCESS));
  DWORD symbol_options = SYMOPT_EXACT_SYMBOLS | SYMOPT_LOAD_LINES;
  SymSetOptions(symbol_options);
  assert(SymInitialize(symbols_process, NULL, FALSE));
#endif
  host = (HostApi){atoi(argv[2]), service};
  M13Thread initializer, worker;
  assert(m13_thread_create(&initializer, initialize_thread, NULL) == 0);
  assert(m13_thread_create(&worker, callback_thread, NULL) == 0);
  load_app(argv[1]);
  ScoopAppError error = dirty_error();
  assert(initialize(NULL, &app, &error) == SCOOP_APP_INIT_INVALID_ARGUMENT);
  empty_error(error);
  error = dirty_error();
  assert(initialize(&host, NULL, &error) == SCOOP_APP_INIT_INVALID_ARGUMENT);
  empty_error(error);
  assert(m13_mutex_lock(&mutex) == 0);
  loaded = 1;
  assert(m13_condition_broadcast(&condition) == 0);
  while (!entered && !returned) {
    assert(m13_condition_wait(&condition, &mutex) == 0);
  }
  assert(m13_mutex_unlock(&mutex) == 0);
  AppApi untouched = {0};
  if (host.mode != 2) {
    error = dirty_error();
    assert(initialize(&host, &untouched, &error) == SCOOP_APP_INIT_IN_PROGRESS);
    empty_error(error);
    assert(untouched.run == NULL && untouched.context == NULL);
  } else {
    assert(!entered);
  }
  assert(m13_mutex_lock(&mutex) == 0);
  proceed = 1;
  assert(m13_condition_broadcast(&condition) == 0);
  assert(m13_mutex_unlock(&mutex) == 0);
  assert(m13_thread_join(initializer) == 0);
  /* No Scoop attachment remains; the initializing OS thread has exited. */
  error = dirty_error();
  if (host.mode == 0) {
    assert(first_status == SCOOP_APP_INIT_OK);
    empty_error(first_error);
    assert(initialize(&host, &untouched, &error) ==
           SCOOP_APP_INIT_ALREADY_INITIALIZED);
    empty_error(error);
    assert(initialize(&host, &untouched, NULL) ==
           SCOOP_APP_INIT_ALREADY_INITIALIZED);
    run_callbacks();
  } else {
    assert(first_status == SCOOP_APP_INIT_FAILED);
    assert(first_error.bytes != NULL && first_error.length > 0);
    assert(initialize(&host, &untouched, &error) == SCOOP_APP_INIT_FAILED);
    assert(error.bytes == first_error.bytes &&
           error.length == first_error.length);
    char *message = malloc((size_t)error.length + 1);
    assert(message != NULL);
    memcpy(message, error.bytes, (size_t)error.length);
    message[error.length] = '\0';
    assert(strstr(message, host.mode == 2 ? "IllegalArgumentException"
                                          : "IllegalStateException") != NULL);
    if (host.mode == 2) {
      assert(strstr(message, "broken") != NULL);
    }
    if (want_trace) {
      assert(strstr(message, "  at ") != NULL);
    }
    free(message);
    assert(initialize(&host, &untouched, NULL) == SCOOP_APP_INIT_FAILED);
  }
  assert(untouched.run == NULL && untouched.context == NULL);
  assert(m13_mutex_lock(&mutex) == 0);
  finished = 1;
  assert(m13_condition_broadcast(&condition) == 0);
  assert(m13_mutex_unlock(&mutex) == 0);
  if (host.mode == 0) {
    assert(m13_mutex_lock(&mutex) == 0);
    while (!blocked) {
      assert(m13_condition_wait(&condition, &mutex) == 0);
    }
    assert(m13_mutex_unlock(&mutex) == 0);
    /* Move roots while another callback is blocked inside its native callee. */
    set_host_environment();
    assert(app.run(4, app.context) == 28);
    check_host_environment();
    assert(m13_mutex_lock(&mutex) == 0);
    collected = 1;
    assert(m13_condition_broadcast(&condition) == 0);
    assert(m13_mutex_unlock(&mutex) == 0);
  }
  assert(m13_thread_join(worker) == 0);
  if (host.mode == 0) {
    /* Release application registrations; the module and runtime stay live. */
    set_host_environment();
    app.close(app.close_context);
    check_host_environment();
    assert(initialize(&host, &untouched, NULL) ==
           SCOOP_APP_INIT_ALREADY_INITIALIZED);
  }
#ifdef _WIN32
  assert(SymGetOptions() == symbol_options);
  assert(SymCleanup(symbols_process));
  assert(CloseHandle(symbols_process));
#endif
  puts(host.mode == 0   ? "native-app-lifecycle-ok"
       : host.mode == 2 ? "native-app-eager-failure-ok"
                        : "native-app-failure-ok");
  return 0;
}
