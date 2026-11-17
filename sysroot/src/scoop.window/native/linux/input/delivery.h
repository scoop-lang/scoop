#ifndef SCOOP_WINDOW_LINUX_INPUT_DELIVERY_H
#define SCOOP_WINDOW_LINUX_INPUT_DELIVERY_H

#include "../window.h"

typedef struct {
  int64_t window;
  GdkEvent *event;
  int32_t kind;
  int64_t contact;
} SWNativeInput;

#endif
