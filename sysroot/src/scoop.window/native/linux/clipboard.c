#include "window.h"

SWClipboardText scoop_window_clipboard_read(void) {
  SWApplication *app = sw_application;
  if (app->error)
    return (SWClipboardText){.error = -3};
  return app->kind == SW_X11 ? sw_x11_clipboard_read(app)
                             : sw_wayland_clipboard_read(app);
}

int64_t scoop_window_clipboard_write(const uint8_t *text, int64_t length) {
  SWApplication *app = sw_application;
  if (app->error)
    return -3;
  return app->kind == SW_X11 ? sw_x11_clipboard_write(app, text, length)
                             : sw_wayland_clipboard_write(app, text, length);
}

void scoop_window_clipboard_release(uint8_t *bytes) { free(bytes); }
