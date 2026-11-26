#pragma once
#include <stdint.h>

typedef struct {
  int32_t (*scene)(void *context, void *scene, int32_t action);
  void *context;
} ScoopUiApi;
