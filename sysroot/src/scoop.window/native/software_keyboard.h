#ifndef SCOOP_WINDOW_SOFTWARE_KEYBOARD_H
#define SCOOP_WINDOW_SOFTWARE_KEYBOARD_H

#include <stdint.h>

typedef struct {
  int32_t state;
  double x, y, width, height;
} SWSoftwareKeyboard;

#endif
