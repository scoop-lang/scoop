#pragma once
#include <stdint.h>

typedef struct {
  double width, height, scale, top, left, bottom, right;
  int32_t pixelsWide, pixelsHigh;
} ScoopWindowMetrics;

typedef struct {
  void (*inspect)(void *window, void *view, ScoopWindowMetrics metrics);
  void (*event)(int32_t kind);
  void (*ime)(int32_t step);
  int32_t textProbe, keyProbe, dragProbe;
} ScoopWindowHost;

typedef struct {
  int32_t (*control)(void *context, int32_t action, void *scene);
  void *context;
} ScoopWindowApi;
