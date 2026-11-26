#pragma once

#include <stdbool.h>

typedef struct {
    bool (*query)(void *context);
    void *context;
} ScoopProtectedDataApi;
