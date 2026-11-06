#include <errno.h>
#include <stdint.h>
#include <sys/eventfd.h>
#include <unistd.h>

int32_t scoop_test_watch_event(void) {
  return eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
}

int32_t scoop_test_watch_signal(int32_t fd) {
  int result;
  do {
    result = eventfd_write(fd, 1);
  } while (result < 0 && errno == EINTR);
  return result;
}

int64_t scoop_test_watch_take(int32_t fd) {
  eventfd_t value;
  int result;
  do {
    result = eventfd_read(fd, &value);
  } while (result < 0 && errno == EINTR);
  return result == 0 ? (int64_t)value : -errno;
}

int32_t scoop_test_watch_close(int32_t fd) { return close(fd); }
