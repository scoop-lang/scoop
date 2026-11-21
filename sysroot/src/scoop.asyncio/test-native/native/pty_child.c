#include <stdint.h>

#ifdef _WIN32
#include "../../../scoop.platform/native/windows_api.h"
#else
#include <sys/ioctl.h>
#include <unistd.h>
#endif

int64_t scoop_test_pty_child_size(void) {
#ifdef _WIN32
    CONSOLE_SCREEN_BUFFER_INFO info;
    if (!GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &info))
        return -1;
    int columns = info.srWindow.Right - info.srWindow.Left + 1;
    int rows = info.srWindow.Bottom - info.srWindow.Top + 1;
#else
    struct winsize size;
    if (!isatty(STDIN_FILENO) || !isatty(STDOUT_FILENO) || !isatty(STDERR_FILENO) ||
        ioctl(STDIN_FILENO, TIOCGWINSZ, &size) < 0)
        return -1;
    int columns = size.ws_col;
    int rows = size.ws_row;
#endif
    return ((int64_t)columns << 32) | rows;
}
