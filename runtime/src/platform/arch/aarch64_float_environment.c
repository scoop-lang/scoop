#include <fenv.h>
#include <stdint.h>
#include <stdlib.h>

#include "../../floating.h"

bool scoop_float_init_environment(void) {
    if (fesetenv(FE_DFL_ENV) != 0) return false;
    /* Clear rounding, traps, flush-to-zero and alternative FP behavior. */
    __asm__ volatile("msr fpcr, %0" : : "r"(UINT64_C(0)));
    return true;
}

uint64_t scoop_runtime_float_enter(fenv_t *saved) {
    uint64_t control;
    __asm__ volatile("mrs %0, fpcr" : "=r"(control));
    if (fegetenv(saved) != 0 || !scoop_float_init_environment()) abort();
    return control;
}

void scoop_runtime_float_leave(const fenv_t *saved, uint64_t control) {
    if (fesetenv(saved) != 0) abort();
    __asm__ volatile("msr fpcr, %0" : : "r"(control));
}
