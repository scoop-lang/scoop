#define _GNU_SOURCE 1
#include "pty.h"
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <signal.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <unistd.h>
#ifdef __APPLE__
#include <pthread.h>
#endif

/* Fork must leave the parent's stdio layout intact, even when a stream is closed. */
static int above_stdio(int descriptor) {
    if (descriptor < 0 || descriptor > STDERR_FILENO)
        return descriptor;
    int copy = fcntl(descriptor, F_DUPFD_CLOEXEC, STDERR_FILENO + 1);
    int error = errno;
    (void)close(descriptor);
    errno = error;
    return copy;
}

static int open_slave(int master) {
    if (grantpt(master) < 0 || unlockpt(master) < 0)
        return -1;
#ifdef __APPLE__
    char name[128];
    if (ioctl(master, TIOCPTYGNAME, name) < 0)
        return -1;
#else
    char name[PATH_MAX];
    int error = ptsname_r(master, name, sizeof(name));
    if (error) {
        errno = error;
        return -1;
    }
#endif
    return above_stdio(open(name, O_RDWR | O_NOCTTY | O_CLOEXEC));
}

static int set_size(int fd, int32_t columns, int32_t rows) {
    struct winsize size = {.ws_col = (unsigned short)columns, .ws_row = (unsigned short)rows};
    int status;
    do {
        status = ioctl(fd, TIOCSWINSZ, &size);
    } while (status < 0 && errno == EINTR);
    return status;
}

static _Noreturn void exec_child(const char *executable, char **argv, char **envp,
                                 const char *directory, int master, int slave, int errors) {
    sigset_t mask;
    (void)sigemptyset(&mask);
    struct sigaction normal = {.sa_handler = SIG_DFL};
    (void)sigemptyset(&normal.sa_mask);
    if (setsid() >= 0 && ioctl(slave, TIOCSCTTY, 0) == 0 &&
        dup2(slave, STDIN_FILENO) >= 0 && dup2(slave, STDOUT_FILENO) >= 0 &&
        dup2(slave, STDERR_FILENO) >= 0 && chdir(directory) == 0 &&
        sigaction(SIGPIPE, &normal, NULL) == 0 && sigprocmask(SIG_SETMASK, &mask, NULL) == 0) {
        (void)close(master);
        (void)close(slave);
        execve(executable, argv, envp);
    }
    int error = errno;
    while (write(errors, &error, sizeof(error)) < 0 && errno == EINTR) {
    }
    _exit(127);
}

static int child_start_result(int descriptor) {
    int error;
    ssize_t count;
    do {
        count = read(descriptor, &error, sizeof(error));
    } while (count < 0 && errno == EINTR);
    if (count == 0)
        return 0;
    if (count < 0)
        return errno;
    return count == sizeof(error) ? error : EIO;
}

static pid_t fork_with_error_pipe(int32_t errors[2]) {
#ifdef __APPLE__
    // Darwin needs pipe and fcntl separately; another PTY spawn must not inherit the write end.
    static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
    (void)pthread_mutex_lock(&lock);
#endif
    pid_t child = scoop_platform_pipe_open(errors) == 0 ? fork() : -1;
#ifdef __APPLE__
    if (child != 0) {
        int error = errno;
        (void)pthread_mutex_unlock(&lock);
        errno = error;
    }
#endif
    return child;
}

ScoopPlatformPtyOpened scoop_platform_pty_spawn(
    const char *executable, char *arguments, int64_t argument_count, char *environment,
    int64_t environment_count, const char *directory, int32_t columns, int32_t rows) {
    ScoopPlatformPty *pty = calloc(1, sizeof(*pty));
    if (!pty)
        return (ScoopPlatformPtyOpened){NULL, ENOMEM};
    pty->process.value = UINT64_MAX;
    pty->input.value = pty->output.value = UINT64_MAX;
    char **argv = scoop_process_string_vector(arguments, argument_count);
    char **envp = NULL;
    int slave = -1;
    int32_t errors[2] = {-1, -1};
    if (!argv)
        goto fail;
    envp = scoop_process_string_vector(environment, environment_count);
    if (!envp)
        goto fail;
    int master = above_stdio(posix_openpt(O_RDWR | O_NOCTTY | O_CLOEXEC | O_NONBLOCK));
    if (master < 0)
        goto fail;
    pty->input = pty->output = scoop_file_wrap(master);
    slave = open_slave(master);
    if (slave < 0 || set_size(slave, columns, rows) < 0)
        goto fail;
    pid_t child = fork_with_error_pipe(errors);
    if (child == 0) {
        (void)close(errors[0]);
        exec_child(executable, argv, envp, directory, master, slave, errors[1]);
    }
    if (child < 0)
        goto fail;
    pty->process.value = (uint64_t)child;
    (void)close(errors[1]);
    errors[1] = -1;
    (void)close(slave);
    slave = -1;
    int error = child_start_result(errors[0]);
    if (error) {
        errno = error;
        goto fail;
    }
    (void)close(errors[0]);
    free(envp);
    free(argv);
    return (ScoopPlatformPtyOpened){pty, 0};

fail:;
    int failure = errno;
    if (errors[0] >= 0)
        (void)close(errors[0]);
    if (errors[1] >= 0)
        (void)close(errors[1]);
    if (slave >= 0)
        (void)close(slave);
    free(envp);
    free(argv);
    (void)scoop_platform_pty_close(pty);
    return (ScoopPlatformPtyOpened){NULL, failure};
}

int32_t scoop_platform_pty_resize(ScoopPlatformPty *pty, int32_t columns, int32_t rows) {
    return set_size(scoop_file_handle(pty->output), columns, rows) < 0 ? errno : 0;
}
