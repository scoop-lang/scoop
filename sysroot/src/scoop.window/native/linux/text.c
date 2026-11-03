#include "window.h"
#include <scoop_text_backend.h>

bool sw_utf8_boundary(const uint8_t *text, size_t length, size_t offset) {
  return offset <= length &&
         (offset == length || (text[offset] & 0xc0) != 0x80);
}

bool sw_utf8_valid(const uint8_t *text, size_t length) {
  for (size_t i = 0; i < length;) {
    uint32_t value = text[i++];
    if (value < 0x80)
      continue;
    size_t extra;
    uint32_t minimum;
    if (value >= 0xc2 && value <= 0xdf) {
      extra = 1;
      minimum = 0x80;
      value &= 0x1f;
    } else if (value >= 0xe0 && value <= 0xef) {
      extra = 2;
      minimum = 0x800;
      value &= 0xf;
    } else if (value >= 0xf0 && value <= 0xf4) {
      extra = 3;
      minimum = 0x10000;
      value &= 7;
    } else
      return false;
    if (extra > length - i)
      return false;
    for (size_t j = 0; j < extra; ++j) {
      uint32_t next = text[i++];
      if ((next & 0xc0) != 0x80)
        return false;
      value = (value << 6) | (next & 0x3f);
    }
    if (value < minimum || value > 0x10ffff ||
        (value >= 0xd800 && value <= 0xdfff))
      return false;
  }
  return true;
}

int64_t sw_utf8_offset(const char *text, size_t length, size_t scalars) {
  size_t index = 0;
  for (size_t count = 0; count < scalars; ++count) {
    if (index == length)
      return -1;
    ++index;
    while (index < length && ((uint8_t)text[index] & 0xc0) == 0x80)
      ++index;
  }
  return (int64_t)index;
}

SWEvent *sw_text_batch(SWWindow *window) {
  SWEvent *event = sw_event(window, 13);
  if (event && window->ime_allowed)
    event->data.a = window->connection;
  return event;
}

bool sw_text_append(SWEvent *event, int kind, const char *text, size_t length,
                    int64_t anchor, int64_t cursor) {
  if (!event || !sw_utf8_valid((const uint8_t *)text, length))
    return false;
  if ((anchor >= 0 &&
       !sw_utf8_boundary((const uint8_t *)text, length, anchor)) ||
      (cursor >= 0 && !sw_utf8_boundary((const uint8_t *)text, length, cursor)))
    return false;
  if (event->edit_count == SIZE_MAX / sizeof(*event->edits))
    return false;
  SWTextEditData *edits =
      realloc(event->edits, (event->edit_count + 1) * sizeof(*edits));
  if (!edits)
    return false;
  event->edits = edits;
  char *copy = malloc(length + 1);
  if (!copy)
    return false;
  memcpy(copy, text, length);
  copy[length] = 0;
  edits[event->edit_count++] = (SWTextEditData){.kind = kind,
                                                .anchor = anchor,
                                                .cursor = cursor,
                                                .text = (uint8_t *)copy,
                                                .text_length = length};
  return true;
}

void sw_text_deliver(SWWindow *window, SWEvent *event) {
  if (!event)
    return;
  if (!event->edit_count) {
    sw_free_event(event);
    return;
  }
  if (event->data.a) {
    if (window->sequence == INT64_MAX) {
      sw_free_event(event);
      sw_fail(window->application, EOVERFLOW);
      return;
    }
    event->data.c = ++window->sequence;
  }
  window->caret_known = false;
  sw_enqueue(window->application, event);
}

void sw_commit(SWWindow *window, const char *text, size_t length) {
  SWEvent *event = sw_text_batch(window);
  if (!sw_text_append(event, 1, text, length, -1, -1)) {
    sw_free_event(event);
    sw_cancel_composition(window);
    return;
  }
  window->preedit_active = false;
  free(window->preedit);
  window->preedit = NULL;
  sw_text_deliver(window, event);
}

void sw_preedit(SWWindow *window, const char *text, size_t length,
                int64_t anchor, int64_t cursor) {
  SWEvent *event = sw_text_batch(window);
  if (!sw_text_append(event, 0, text, length, anchor, cursor)) {
    sw_free_event(event);
    sw_cancel_composition(window);
    return;
  }
  char *copy = strndup(text, length);
  if (!copy) {
    sw_free_event(event);
    sw_fail(window->application, ENOMEM);
    return;
  }
  free(window->preedit);
  window->preedit = copy;
  window->preedit_active = length != 0;
  sw_text_deliver(window, event);
}

void sw_cancel_composition(SWWindow *window) {
  if (window->compose)
    xkb_compose_state_reset(window->compose);
  if (window->preedit_active) {
    window->preedit_active = false;
    sw_preedit(window, "", 0, 0, 0);
  }
}

int32_t scoop_window_ime_allowed(SWWindow *window, int32_t allowed) {
  if (window->ime_allowed == (allowed != 0))
    return 0;
  if (allowed && window->connection == INT64_MAX)
    return EOVERFLOW;
  sw_cancel_composition(window);
  window->ime_allowed = allowed != 0;
  window->caret_known = false;
  if (allowed) {
    ++window->connection;
    window->sequence = window->applied_sequence = 0;
    free((void *)window->document.text);
    window->document = (SWTextSnapshotData){.revision = -1};
  }
  if (window->application->kind == SW_X11)
    sw_x11_ime(window);
  else
    sw_wayland_ime(window);
  SWEvent *event = sw_event(window, 14);
  if (!event)
    return ENOMEM;
  event->data.flags = allowed;
  event->data.a = window->connection;
  sw_enqueue(window->application, event);
  return 0;
}

int64_t scoop_window_ime_caret(SWWindow *window, double x, double y,
                               double width, double height) {
  window->caret_x = x;
  window->caret_y = y;
  window->caret_width = width;
  window->caret_height = height;
  window->caret_known = true;
  if (window->application->kind == SW_X11)
    sw_x11_caret(window);
  else
    sw_wayland_caret(window);
  return 0;
}

int64_t scoop_window_clear_ime_caret(SWWindow *window) {
  window->caret_known = false;
  if (window->application->kind == SW_WAYLAND)
    sw_wayland_caret(window);
  return 0;
}

int64_t scoop_window_publish_text(SWWindow *window, SWTextSnapshotData data) {
  if (!window->ime_allowed || window->connection != data.connection)
    return 1;
  if (data.after < 0 || data.after > window->sequence ||
      (data.after && data.after < window->applied_sequence) ||
      data.revision < window->document.revision)
    return 2;
  if (data.length < 0 || data.anchor < 0 || data.cursor < 0 ||
      !sw_utf8_valid(data.text, data.length) ||
      !sw_utf8_boundary(data.text, data.length, data.anchor) ||
      !sw_utf8_boundary(data.text, data.length, data.cursor) ||
      (data.marked_start >= 0 &&
       (data.marked_end < data.marked_start ||
        !sw_utf8_boundary(data.text, data.length, data.marked_start) ||
        !sw_utf8_boundary(data.text, data.length, data.marked_end))))
    return 2;
  SWTextSnapshotData previous = window->document;
  if (previous.revision == data.revision &&
      (previous.length != data.length || previous.anchor != data.anchor ||
       previous.cursor != data.cursor ||
       previous.marked_start != data.marked_start ||
       previous.marked_end != data.marked_end ||
       memcmp(previous.text, data.text, data.length)))
    return 2;
  uint8_t *copy = malloc(data.length + 1);
  if (!copy)
    return ENOMEM;
  memcpy(copy, data.text, data.length);
  copy[data.length] = 0;
  window->document = data;
  window->document.text = copy;
  free((void *)previous.text);
  if (data.after)
    window->applied_sequence = data.after;
  if (previous.revision != data.revision)
    window->caret_known = false;
  if (window->application->kind == SW_WAYLAND)
    sw_wayland_document(window);
  return 0;
}

int64_t scoop_window_publish_text_caret(SWWindow *window, int64_t connection,
                                        int64_t revision, bool available,
                                        double x, double y, double width,
                                        double height) {
  if (!window->ime_allowed || window->connection != connection ||
      window->document.revision != revision ||
      window->applied_sequence != window->sequence)
    return 1;
  if (available)
    return scoop_window_ime_caret(window, x, y, width, height);
  return scoop_window_clear_ime_caret(window);
}

int32_t scoop_window_shortcut_equal(const uint8_t *left, int64_t left_size,
                                    const uint8_t *right, int64_t right_size) {
  scoop_text_result a = scoop_charset_case(left, left_size, 2);
  scoop_text_result b = scoop_charset_case(right, right_size, 2);
  scoop_text_result na = {0}, nb = {0};
  int result = -1;
  if (!a.code && !b.code) {
    na = scoop_charset_normalize_nfc(a.data, a.size);
    nb = scoop_charset_normalize_nfc(b.data, b.size);
    if (!na.code && !nb.code)
      result = na.size == nb.size &&
               (!na.size || !memcmp(na.data, nb.data, na.size));
  }
  scoop_text_free(a.data, a.size);
  scoop_text_free(b.data, b.size);
  scoop_text_free(na.data, na.size);
  scoop_text_free(nb.data, nb.size);
  return result;
}
