#ifndef SCOOP_RT_FLOATING_H
#define SCOOP_RT_FLOATING_H

#include <fenv.h>
#include <stdbool.h>
#include <stdint.h>

typedef struct ScoopFloatEnvironment {
    fenv_t environment;
    uint64_t control;
} ScoopFloatEnvironment;

/* Establish Scoop's environment whenever native code returns to managed code. */
bool scoop_float_init_environment(void);

/* Private C ABI used by generated static callbacks and managed gateways. */
uint64_t scoop_runtime_float_enter(fenv_t *saved);
void scoop_runtime_float_leave(const fenv_t *saved, uint64_t control);

#endif /* SCOOP_RT_FLOATING_H */
