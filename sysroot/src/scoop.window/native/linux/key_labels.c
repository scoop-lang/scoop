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
  if (!app->keymap || !app->key_state || scan > UINT32_MAX - 8)
    return (SWKeyLabel){0};
  xkb_keycode_t code = scan + 8;
  xkb_layout_index_t layout = xkb_state_key_get_layout(app->key_state, code);
  if (layout == XKB_LAYOUT_INVALID)
    return (SWKeyLabel){0};
  const xkb_keysym_t *symbols;
  int count =
      xkb_keymap_key_get_syms_by_level(app->keymap, code, layout, 0, &symbols);
  if (count != 1)
    return (SWKeyLabel){0};
  SWKeyLabel result = {.symbol = symbols[0]};
  char text[64];
  int size = xkb_keysym_to_utf8(symbols[0], text, sizeof(text));
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
