#pragma once
#include <stdint.h>

typedef struct {
  void (*report)(int32_t stage);
} ScoopSurfaceHost;

typedef struct {
  int32_t (*scene)(void *context, void *scene, int32_t action);
  void *context;
} ScoopSurfaceApi;
