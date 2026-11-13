#include <stdint.h>

#if defined(__ELF__)
extern const unsigned char __ehdr_start[];
#endif

int32_t m23_global = 2;
int32_t m23_target(int32_t value) { return value + 1; }
int32_t (*m23_pointer)(int32_t) = m23_target;
static int32_t local_call(int32_t value) { return m23_target(value); }
int32_t m23_final(int32_t value) {
#if defined(__ELF__)
    if (__ehdr_start[0] != 0x7f || __ehdr_start[1] != 'E' ||
        __ehdr_start[2] != 'L' || __ehdr_start[3] != 'F') {
        return -1;
    }
#endif
    return local_call(value) + m23_global + m23_pointer(value);
}
