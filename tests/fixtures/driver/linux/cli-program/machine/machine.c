#include <stdbool.h>

bool fixture_linux_is_aarch64(void) {
#if defined(__aarch64__)
    return true;
#elif defined(__x86_64__)
    return false;
#else
#error "No fixture machine name for this architecture"
#endif
}
