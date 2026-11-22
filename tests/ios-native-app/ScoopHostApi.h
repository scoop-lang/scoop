#ifndef SCOOP_TEST_HOST_API_H
#define SCOOP_TEST_HOST_API_H

#include <stdint.h>

typedef struct ScoopTestHostApi {
    int32_t (*service)(int32_t value);
    int32_t fail;
} ScoopTestHostApi;

typedef struct ScoopTestAppApi {
    int32_t (*run)(int32_t value, void *context);
    void *context;
    void (*close)(void *context);
    void *close_context;
} ScoopTestAppApi;

#endif
