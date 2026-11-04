#include "window.h"

#include <string.h>

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWKeyLabel;

static int normalize(const WCHAR *text, int length, WCHAR **result,
                     int *result_length) {
  int count = FoldStringW(MAP_PRECOMPOSED, text, length, NULL, 0);
  if (!count)
    return 0;
  WCHAR *value = malloc((size_t)count * sizeof(WCHAR));
  if (!value)
    return 0;
  if (FoldStringW(MAP_PRECOMPOSED, text, length, value, count) != count) {
    free(value);
    return 0;
  }
  *result = value;
  *result_length = count;
  return 1;
}

int32_t scoop_window_shortcut_equal(const uint8_t *left, int64_t left_size,
                                    const uint8_t *right, int64_t right_size) {
  if (left_size < 0 || left_size > INT_MAX || right_size < 0 ||
      right_size > INT_MAX)
    return -1;
  WCHAR *a = sw_wide(left, left_size), *b = sw_wide(right, right_size);
  if (!a || !b) {
    free(a);
    free(b);
    return -1;
  }
  int a_count = (int)wcslen(a), b_count = (int)wcslen(b);
  WCHAR *folded_a = NULL, *folded_b = NULL;
  int folded_a_count = 0, folded_b_count = 0;
  bool ok = normalize(a, a_count, &folded_a, &folded_a_count) &&
            normalize(b, b_count, &folded_b, &folded_b_count);
  int32_t result = -1;
  if (ok) {
    int compared = CompareStringOrdinal(folded_a, folded_a_count, folded_b,
                                        folded_b_count, TRUE);
    if (compared)
      result = compared == CSTR_EQUAL;
  }
  free(folded_a);
  free(folded_b);
  free(a);
  free(b);
  return result;
}

SWKeyLabel scoop_window_key_label(uint32_t scan) {
  if (!scan || scan > 511)
    return (SWKeyLabel){.error = 1};
  UINT virtual_key = MapVirtualKeyW(scan & 0xff, MAPVK_VSC_TO_VK_EX);
  if (!virtual_key)
    return (SWKeyLabel){0};
  BYTE state[256] = {0};
  WCHAR text[8];
  int count = ToUnicodeEx(virtual_key, scan & 0xff, state, text, 8, 4,
                          GetKeyboardLayout(0));
  if (!count)
    return (SWKeyLabel){0};
  if (count < 0)
    count = -count;
  int64_t length = 0;
  uint8_t *bytes = sw_utf8(text, count, &length);
  if (!bytes)
    return (SWKeyLabel){.error = 2};
  return (SWKeyLabel){bytes, length, 0};
}

void scoop_window_key_label_release(uint8_t *bytes) { free(bytes); }
