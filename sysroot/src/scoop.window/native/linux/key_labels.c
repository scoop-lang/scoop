#include "window.h"

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
  uint32_t symbol;
} SWKeyLabel;
SWKeyLabel scoop_window_key_label(uint32_t scan) {
  SWApplication *app = sw_application;
  if (!app)
    return (SWKeyLabel){.error = EINVAL};
  if (scan > UINT32_MAX - 8)
    return (SWKeyLabel){0};
  GdkSeat *seat = gdk_display_get_default_seat(app->display);
  GdkDevice *keyboard = seat ? gdk_seat_get_keyboard(seat) : NULL;
  int active_layout =
      keyboard ? gdk_device_get_active_layout_index(keyboard) : -1;
  guint symbol;
  int group, level;
  GdkModifierType consumed;
  if (!gdk_display_translate_key(app->display, scan + 8, 0,
                                 active_layout >= 0 ? active_layout
                                                    : app->layout,
                                 &symbol, &group, &level, &consumed))
    return (SWKeyLabel){0};
  SWKeyLabel result = {.symbol = symbol};
  char text[64];
  int size = xkb_keysym_to_utf8(symbol, text, sizeof(text));
  if (size <= 1 || (uint8_t)text[0] < 32 || (uint8_t)text[0] == 127)
    return result;
  result.bytes = (uint8_t *)strndup(text, size - 1);
  if (!result.bytes)
    result.error = ENOMEM;
  else
    result.length = size - 1;
  return result;
}
void scoop_window_key_label_release(uint8_t *bytes) { free(bytes); }
