#include <errno.h>
#include <stdint.h>
#include <sys/eventfd.h>
#include <unistd.h>

typedef struct {
    uint64_t value;
    int32_t status;
} EventRead;

int32_t scoop_os_linux_event_open(void) { return eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC); }

// Zero is success, one is nonblocking backpressure, and a negative status preserves errno.
static int32_t event_status(int result) {
    if (result == 0)
        return 0;
    return errno == EAGAIN ? 1 : -errno;
}

int32_t scoop_os_linux_event_add(int32_t fd, uint64_t value) {
    int result;
    do {
        result = eventfd_write(fd, value);
    } while (result < 0 && errno == EINTR);
    return event_status(result);
}

EventRead scoop_os_linux_event_take(int32_t fd) {
    eventfd_t value = 0;
    int result;
    do {
        result = eventfd_read(fd, &value);
    } while (result < 0 && errno == EINTR);
    return (EventRead){value, event_status(result)};
}
