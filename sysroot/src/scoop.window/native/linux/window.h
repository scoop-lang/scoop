#ifndef SCOOP_WINDOW_LINUX_H
#define SCOOP_WINDOW_LINUX_H

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
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
#define GDK_VERSION_MIN_REQUIRED GDK_VERSION_4_14
#define GDK_VERSION_MAX_ALLOWED GDK_VERSION_4_18
#define ADW_VERSION_MIN_REQUIRED ADW_VERSION_1_4
#define ADW_VERSION_MAX_ALLOWED ADW_VERSION_1_4
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <adwaita.h>
#pragma clang diagnostic pop
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
  double work_x, work_y, work_width, work_height;
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
typedef struct SWMenu SWMenu;
typedef struct {
  SWCallback callback;
  void *context;
  GMainContext *main_context;
  GdkDisplay *display;
  GListModel *monitors;
  gulong display_closed, monitors_changed, theme_changed;
  int kind, wake_fd, error, layout;
  guint gdk_modifiers;
  int64_t next_monitor;
  bool stopped, error_reported, alt_graph;
  double deadline;
  SWEvent *head, *tail;
  SWWindow *windows;
  SWWatch *watches;
  bool physical_keys[768], physical_keys_known;
} SWApplication;
struct SWWindow {
  SWApplication *application;
  SWWindow *next;
  GtkWidget *window, *view, *toolbar, *header;
  SWMenu *menu_bar;
  GtkIMContext *ime, *simple_ime;
  bool destroying;
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
void sw_set_geometry(SWWindow *window, double width, double height,
                     double scale);
int64_t sw_transparency(SWWindow *window, bool transparent);
uint64_t sw_modifiers(SWApplication *app);
void sw_update_modifiers(SWWindow *window);
void sw_input_install(SWWindow *window);
bool sw_menu_key_equivalent(SWWindow *window, guint symbol);
void sw_ime_install(SWWindow *window);
void sw_ime_update(SWWindow *window);
void sw_ime_caret(SWWindow *window);
void sw_ime_document(SWWindow *window);
GtkIMContext *sw_ime_context(SWWindow *window);
void sw_snapshot_update(SWWindow *window);
void sw_desktop_bounds(SWWindow *window);
void sw_update_buttons(SWWindow *window);
int64_t sw_request_size(SWWindow *window, double width, double height);
GdkSurface *sw_surface(SWWindow *window);
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

void sw_monitors_changed(GListModel *model, guint position, guint removed,
                         guint added, gpointer data);
void sw_theme_changed(AdwStyleManager *manager, GParamSpec *pspec,
                      gpointer data);

#endif
