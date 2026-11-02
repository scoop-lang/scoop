#include <fenv.h>
#include <stdint.h>
#include <stdlib.h>
#include <xmmintrin.h>

#include "../../floating.h"

bool scoop_float_init_environment(void) {
    if (fesetenv(FE_DFL_ENV) != 0) return false;
    /* Mask all SSE traps, round to nearest and retain all subnormal values. */
    _mm_setcsr(UINT32_C(0x1f80));
    return true;
}

uint64_t scoop_runtime_float_enter(fenv_t *saved) {
    uint64_t control = _mm_getcsr();
    if (fegetenv(saved) != 0 || !scoop_float_init_environment()) abort();
    return control;
}

void scoop_runtime_float_leave(const fenv_t *saved, uint64_t control) {
    if (fesetenv(saved) != 0) abort();
    _mm_setcsr((uint32_t)control);
}
