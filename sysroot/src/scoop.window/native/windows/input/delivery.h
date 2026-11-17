#ifndef SCOOP_WINDOW_WINDOWS_INPUT_DELIVERY_H
#define SCOOP_WINDOW_WINDOWS_INPUT_DELIVERY_H

#include "../window.h"

typedef struct {
  int64_t window;
  int32_t kind, button;
  POINT position;
  int64_t contact;
} SWNativeInput;

#endif
