#ifndef SCOOP_WINDOW_WINDOWS_INPUT_STATE_H
#define SCOOP_WINDOW_WINDOWS_INPUT_STATE_H

#include "../../contacts.h"
#include "../../input.h"
#include "clicks.h"
#include <windows.h>

typedef struct {
  SWContactEntry *contacts;
  uint64_t modifiers;
  SWKeyboardData pressed[512];
  bool key_down[512];
  uint32_t pressed_buttons;
  int pressed_clicks[32];
  SWClicks clicks;
  bool cursor_visible, pointer_inside;
  HCURSOR cursor;
} SWInputState;

#endif
