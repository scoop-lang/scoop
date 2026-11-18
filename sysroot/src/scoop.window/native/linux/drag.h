#ifndef SCOOP_WINDOW_LINUX_DRAG_H
#define SCOOP_WINDOW_LINUX_DRAG_H

#include "../drag_drop.h"
#include "window.h"

bool sw_drag_absolute_uri(const char *value);
int sw_drag_local(SWWindow *window, GdkDrop *drop);
int sw_drag_current_effects(SWWindow *window, GdkDrop *drop, int fallback);
GPtrArray *sw_drop_decode(const uint8_t *bytes, size_t length, int format);

#endif
