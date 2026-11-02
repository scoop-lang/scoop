#ifndef SCOOP_APP_H
#define SCOOP_APP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct ScoopAppError {
    const uint8_t *bytes;
    uint64_t length;
} ScoopAppError;

enum {
    SCOOP_APP_INIT_OK = 0,
    SCOOP_APP_INIT_ALREADY_INITIALIZED = 1,
    SCOOP_APP_INIT_IN_PROGRESS = 2,
    SCOOP_APP_INIT_FAILED = 3,
    SCOOP_APP_INIT_INVALID_ARGUMENT = 4,
};

/* Call after loading, outside loader callbacks. Keep the module loaded until
 * process exit. Read the output table only after OK. Error bytes are borrowed
 * for the module lifetime and must not be modified or freed. */
int32_t scoop_app_init(const void *host_api, void *out_app_api, ScoopAppError *out_error);

#ifdef __cplusplus
}
#endif

#endif
