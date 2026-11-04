#ifndef SCOOP_WINDOW_LINUX_H
#define SCOOP_WINDOW_LINUX_H

#define _GNU_SOURCE
#include <errno.h>
#include <limits.h>
#include <math.h>
#include <poll.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <xkbcommon/xkbcommon-compose.h>
#include <xkbcommon/xkbcommon.h>

typedef int32_t (*SWCallback)(void *, int64_t, int32_t);
typedef struct {
  int64_t window;
  int32_t kind, flags;
  int64_t a, b, c, d;
  double x, y, z, w, scale, dx, dy;
  const uint8_t *text;
  int64_t text_length, time_seconds, time_nanoseconds;
} SWEventData;
typedef struct {
  int32_t kind;
  int64_t anchor, cursor;
  const uint8_t *text;
  int64_t text_length;
  int32_t has_replacement;
  int64_t replacement_start, replacement_end;
} SWTextEditData;
typedef struct {
  double width, height, pixel_width, pixel_height, scale;
  double x, y, outer_width, outer_height;
  int32_t visible, occluded, focused, minimized, maximized, dark;
} SWSnapshot;
typedef struct {
  void *pointer;
  int64_t error;
} SWResult;
typedef struct {
  int64_t key, primary, refresh_millihertz;
  double x, y, width, height, pixel_width, pixel_height, scale;
  const uint8_t *name;
  int64_t name_length;
} SWMonitorData;
typedef struct {
  SWMonitorData *items;
  size_t count;
} SWMonitors;
typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWClipboardText;
typedef struct {
  int64_t connection, revision, after;
  const uint8_t *text;
  int64_t length, anchor, cursor, marked_start, marked_end;
} SWTextSnapshotData;
typedef struct SWEvent {
  SWEventData data;
  SWTextEditData *edits;
  size_t edit_count;
  struct SWEvent *next;
} SWEvent;
typedef struct SWWatch SWWatch;
typedef struct SWWindow SWWindow;
typedef struct {
  SWCallback callback;
  void *context, *backend;
  int kind, wake_fd, display_fd, decor_fd, error;
  bool stopped, error_reported, display_write_pending;
  double deadline;
  SWEvent *head, *tail;
  SWWindow *windows;
  SWWatch *watches;
  struct xkb_context *xkb;
  struct xkb_keymap *keymap;
  struct xkb_state *key_state;
  struct xkb_compose_table *compose_table;
  bool physical_keys[768], physical_keys_known;
} SWApplication;
struct SWWindow {
  SWApplication *application;
  SWWindow *next;
  void *backend;
  int64_t identity;
  SWSnapshot snapshot;
  bool configured, requested_visible, decorations, resizable, transparent;
  bool close_button, minimize_button, maximize_button, cursor_visible;
  bool pointer_inside, ime_allowed, preedit_active, caret_known;
  int cursor_icon;
  double requested_width, requested_height;
  double min_width, min_height, max_width, max_height;
  double increment_width, increment_height;
  double caret_x, caret_y, caret_width, caret_height;
  double pointer_x, pointer_y;
  double click_time;
  double click_x, click_y;
  int click_button, click_count;
  uint64_t modifiers;
  uint32_t pressed_buttons;
  SWEventData pressed[768];
  bool key_down[768];
  struct xkb_compose_state *compose;
  char *preedit;
  SWTextSnapshotData document;
  int64_t connection, sequence, applied_sequence;
};

enum { SW_X11 = 1, SW_WAYLAND = 2 };
enum {
  SW_SHIFT = 1,
  SW_CONTROL = 2,
  SW_ALT = 4,
  SW_SUPER = 8,
  SW_CAPS = 16,
  SW_NUM = 32,
  SW_SCROLL = 64,
  SW_ALT_GRAPH = 128,
  SW_ALT_GRAPH_KNOWN = 1u << 17
};
extern SWApplication *sw_application;
double sw_now(void);
void sw_wake(SWApplication *app);
void sw_fail(SWApplication *app, int error);
SWEvent *sw_event(SWWindow *window, int kind);
void sw_enqueue(SWApplication *app, SWEvent *event);
void sw_free_event(SWEvent *event);
void sw_emit(SWWindow *window, int kind);
void sw_stamp_input(SWEventData *event);
void sw_watch_dispatch(SWApplication *app, struct pollfd *fds, size_t count,
                       SWWatch **watches);
void sw_watch_retire(SWApplication *app);
size_t sw_watch_count(SWApplication *app);
void sw_watch_poll(SWApplication *app, struct pollfd *fds, SWWatch **watches);
int sw_dispatch_native(SWApplication *app);
int sw_flush(SWApplication *app);
void sw_set_geometry(SWWindow *window, double width, double height,
                     double scale);
int64_t sw_transparency(SWWindow *window, bool transparent);
uint64_t sw_modifiers(SWApplication *app);
void sw_update_modifiers(SWWindow *window);
void sw_key(SWWindow *window, uint32_t code, bool down, bool repeat, bool text);
void sw_reset_input(SWWindow *window);
void sw_focus(SWWindow *window, bool focused);
void sw_pointer(SWWindow *window, int kind, int64_t button, double dx,
                double dy, int phase);
void sw_cancel_pointer(SWWindow *window, int reason);
void sw_cancel_composition(SWWindow *window);
SWEvent *sw_text_batch(SWWindow *window);
bool sw_text_append(SWEvent *event, int kind, const char *text, size_t length,
                    int64_t anchor, int64_t cursor);
void sw_text_deliver(SWWindow *window, SWEvent *event);
void sw_commit(SWWindow *window, const char *text, size_t length);
void sw_preedit(SWWindow *window, const char *text, size_t length,
                int64_t anchor, int64_t cursor);
bool sw_utf8_valid(const uint8_t *text, size_t length);
bool sw_utf8_boundary(const uint8_t *text, size_t length, size_t offset);
int64_t sw_utf8_offset(const char *text, size_t length, size_t scalars);

int sw_x11_initialize(SWApplication *app);
void sw_x11_finish(SWApplication *app);
int sw_x11_dispatch(SWApplication *app);
int sw_x11_flush(SWApplication *app);
int sw_x11_create(SWWindow *window, const char *title);
void sw_x11_destroy(SWWindow *window);
int64_t sw_x11_title(SWWindow *window, const char *title);
int64_t sw_x11_size(SWWindow *window, double width, double height);
int64_t sw_x11_visible(SWWindow *window, bool visible);
int64_t sw_x11_focus(SWWindow *window);
int64_t sw_x11_position(SWWindow *window, double x, double y);
int64_t sw_x11_hints(SWWindow *window);
int64_t sw_x11_level(SWWindow *window, int value);
int64_t sw_x11_minimized(SWWindow *window, bool value);
int64_t sw_x11_maximized(SWWindow *window, bool value);
int64_t sw_x11_theme(SWWindow *window, int value);
int64_t sw_x11_attention(SWWindow *window, int value);
int64_t sw_x11_cursor(SWWindow *window);
SWMonitors *sw_x11_monitors(SWApplication *app);
int64_t sw_x11_current_monitor(SWWindow *window);
void sw_x11_ime(SWWindow *window);
void sw_x11_caret(SWWindow *window);
SWClipboardText sw_x11_clipboard_read(SWApplication *app);
int64_t sw_x11_clipboard_write(SWApplication *app, const uint8_t *text,
                               size_t length);
void *sw_x11_connection(SWWindow *window);
uint32_t sw_x11_window(SWWindow *window);
uint32_t sw_x11_visual(SWWindow *window);

int sw_wayland_initialize(SWApplication *app);
void sw_wayland_finish(SWApplication *app);
int sw_wayland_dispatch(SWApplication *app);
int sw_wayland_flush(SWApplication *app);
int sw_wayland_create(SWWindow *window, const char *title);
void sw_wayland_destroy(SWWindow *window);
int64_t sw_wayland_title(SWWindow *window, const char *title);
int64_t sw_wayland_size(SWWindow *window, double width, double height);
int64_t sw_wayland_visible(SWWindow *window, bool visible);
int64_t sw_wayland_focus(SWWindow *window);
int64_t sw_wayland_hints(SWWindow *window);
int64_t sw_wayland_minimized(SWWindow *window, bool value);
int64_t sw_wayland_maximized(SWWindow *window, bool value);
int64_t sw_wayland_cursor(SWWindow *window);
SWMonitors *sw_wayland_monitors(SWApplication *app);
int64_t sw_wayland_current_monitor(SWWindow *window);
void sw_wayland_ime(SWWindow *window);
void sw_wayland_caret(SWWindow *window);
void sw_wayland_document(SWWindow *window);
double sw_wayland_repeat_deadline(SWApplication *app);
void sw_wayland_repeat(SWApplication *app);
SWClipboardText sw_wayland_clipboard_read(SWApplication *app);
int64_t sw_wayland_clipboard_write(SWApplication *app, const uint8_t *text,
                                   size_t length);
void *sw_wayland_display(SWWindow *window);
void *sw_wayland_surface(SWWindow *window);
size_t sw_wayland_transfer_count(SWApplication *app);
void sw_wayland_transfer_poll(SWApplication *app, struct pollfd *fds);
void sw_wayland_transfer_flush(SWApplication *app);

#endif
