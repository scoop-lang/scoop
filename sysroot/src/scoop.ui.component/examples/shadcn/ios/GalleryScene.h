#pragma once
#include <stdint.h>

typedef struct {
    int32_t (*scene)(void *context, void *windowScene, int32_t operation);
    void *context;
} ScoopGalleryApi;

enum { ScoopGalleryConnect = 1, ScoopGalleryDisconnect = 2 };
