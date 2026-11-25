#define _POSIX_C_SOURCE 200809L
#include "test_process.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>
#ifdef __APPLE__
#include <TargetConditionals.h>
#include <mach-o/dyld.h>
#endif

#if !defined(__APPLE__) || !TARGET_OS_IPHONE
#include <spawn.h>
extern char **environ;

static char **child_environment(const char *path, char **setting) {
    const char prefix[] = "SCOOP_TEST_CASE_RESULT=";
    size_t length = sizeof(prefix) + strlen(path);
    *setting = malloc(length);
    if (!*setting)
        return NULL;
    snprintf(*setting, length, "%s%s", prefix, path);
    size_t count = 0;
    while (environ[count])
        count++;
    char **values = calloc(count + 2, sizeof(*values));
    if (!values)
        return NULL;
    size_t used = 0;
    for (size_t i = 0; i < count; ++i) {
        if (strncmp(environ[i], prefix, sizeof(prefix) - 1) &&
            strncmp(environ[i], "SCOOP_TEST_RUN_DIR=", sizeof("SCOOP_TEST_RUN_DIR=") - 1))
            values[used++] = environ[i];
    }
    values[used] = *setting;
    return values;
}

static int start_child(pid_t *pid, const char *key, const char *path, FILE *output, FILE *error) {
#ifdef __APPLE__
    uint32_t length = 0;
    _NSGetExecutablePath(NULL, &length);
    char *image = malloc(length);
    if (!image)
        return ENOMEM;
    if (_NSGetExecutablePath(image, &length)) {
        free(image);
        return ENOENT;
    }
#else
    char image[] = "/proc/self/exe";
#endif
    char *setting = NULL;
    char **env = child_environment(path, &setting);
    posix_spawn_file_actions_t actions;
    int err = env ? posix_spawn_file_actions_init(&actions) : ENOMEM;
    if (!err) {
        if (output)
            err = posix_spawn_file_actions_adddup2(&actions, fileno(output), STDOUT_FILENO);
        if (!err && error)
            err = posix_spawn_file_actions_adddup2(&actions, fileno(error), STDERR_FILENO);
        if (!err && output)
            err = posix_spawn_file_actions_addclose(&actions, fileno(output));
        if (!err && error)
            err = posix_spawn_file_actions_addclose(&actions, fileno(error));
        char *argv[] = {image, "--scoop-test-child", (char *)key, NULL};
        if (!err)
            err = posix_spawn(pid, image, &actions, NULL, argv, env);
        posix_spawn_file_actions_destroy(&actions);
    }
    free(env);
    free(setting);
#ifdef __APPLE__
    free(image);
#endif
    return err;
}
#endif

int scoop_test_spawn(const char *key, const char *path, FILE *output, FILE *error,
                     struct TestExit *status, char *reason, size_t capacity) {
#if defined(__APPLE__) && TARGET_OS_IPHONE
    snprintf(reason, capacity, "this iOS target does not support spawning a test child");
    return 1;
#else
    pid_t pid;
    int err = start_child(&pid, key, path, output, error);
    if (err) {
        snprintf(reason, capacity, "cannot start test child: %s", strerror(err));
        return 1;
    }
    int code;
    pid_t waited;
    do {
        waited = waitpid(pid, &code, 0);
    } while (waited < 0 && errno == EINTR);
    if (waited < 0 || (!WIFEXITED(code) && !WIFSIGNALED(code))) {
        snprintf(reason, capacity, "cannot wait for test child: %s", strerror(errno));
        return -1;
    }
    if (WIFEXITED(code))
        *status = (struct TestExit){1, WEXITSTATUS(code), 0};
    else
        *status = (struct TestExit){0, 0, WTERMSIG(code)};
    return 0;
#endif
}
