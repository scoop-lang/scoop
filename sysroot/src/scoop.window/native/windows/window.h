#ifndef SCOOP_WINDOW_WINDOWS_H
#define SCOOP_WINDOW_WINDOWS_H

#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#define UNICODE
#include <windows.h>

#include "../frame.h"
#include "input/state.h"
#include <dwmapi.h>
#include <imm.h>
#include <limits.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <windowsx.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int32_t (*SWCallback)(void *, int64_t, int32_t);
#include "../events.h"
typedef struct {
  double width, height, pixel_width, pixel_height, scale;
  double x, y, outer_width, outer_height;
  int32_t visible, occluded, focused, minimized, maximized, fullscreen, dark;
} SWSnapshot;
typedef struct {
  void *pointer;
  int64_t error;
} SWResult;
typedef struct {
  int64_t connection, revision, after;
  const uint8_t *text;
  int64_t length, anchor, cursor, marked_start, marked_end;
} SWTextSnapshotData;
typedef struct SWEvent {
  SWEventHeader header;
  union {
    SWEventData data;
    SWKeyboardData keyboard;
    SWPointerData pointer;
    SWGestureData gesture;
    SWContactData contact;
    int64_t modifiers;
  };
  bool native_input;
  POINT input_position;
  SWTextEditData *edits;
  int64_t edit_count, edit_capacity;
  struct SWEvent *next;
} SWEvent;
typedef struct SWWatch SWWatch;
extern UINT sw_taskbar_created_message;
typedef struct SWMenu SWMenu;
typedef struct SWApplication {
  SWCallback callback;
  void *context;
  HWND hwnd;
  HANDLE wake;
  volatile LONG pending, error;
  bool stopped;
  double deadline;
  SWEvent *head, *tail;
  SWWatch *watches;
  void *text_manager;
  void *preferences;
  void *drag_source;
  HWND promoted_drag_window;
  bool ole_initialized, compat_delivery;
  SWMenu *translating_menu;
} SWApplication;
typedef struct SWWindow {
  bool taskbar_ready;
  SWApplication *application;
  struct SWWindow *owning_window;
  HWND hwnd;
  int64_t identity;
  double scale, min_width, min_height, max_width, max_height;
  double increment_width, increment_height;
  int frame_mode;
  SWHitRect *hit_rects;
  size_t hit_count;
  double frame_width, frame_height, frame_scale;
  uint64_t frame_revision;
  int frame_hover, frame_pressed, frame_button_down;
  bool window_region;
  bool fullscreen;
  WINDOWPLACEMENT fullscreen_placement;
  HMENU fullscreen_menu;
  bool resizable, close_button, minimize_button, maximize_button;
  bool transparent, bottom, ime_allowed;
  int32_t menu_text_commands;
  bool destroying, caret_set;
  bool input_blocked, saved_enabled;
  int theme;
  double caret_x, caret_y, caret_width, caret_height;
  WCHAR surrogate;
  SWInputState input;
  int64_t text_connection;
  void *text;
  void *accessibility;
  void *drop_target;
  void *keyboard;
  SWMenu *menu_bar, *popup_menu;
} SWWindow;

int32_t scoop_window_ime_allowed(SWWindow *window, int32_t allowed);
int64_t scoop_window_ime_caret(SWWindow *window, double x, double y,
                               double width, double height);
int64_t scoop_window_publish_text(void *pointer, SWTextSnapshotData data);
int64_t scoop_window_publish_text_caret(void *pointer, int64_t connection,
                                        int64_t revision, bool available,
                                        double x, double y, double width,
                                        double height);
int64_t scoop_window_clear_ime_caret(void *pointer);
int64_t scoop_window_text_count(SWEvent *event);
SWTextEditData scoop_window_text_edit(SWEvent *event, int64_t index);

enum {
  SW_WAKE = WM_APP + 1,
  SW_READY,
  SW_TEXT_LOCK,
  SW_PREFERENCES,
  SW_ACCESSIBILITY
};

bool sw_accessibility_message(SWWindow *window, UINT message, WPARAM wparam,
                              LPARAM lparam, LRESULT *result);
void sw_accessibility_destroy(SWWindow *window);

int64_t sw_preferences_start(SWApplication *app);
void sw_preferences_stop(SWApplication *app);
extern const WCHAR sw_window_class[];
void sw_wake(SWApplication *application);
void sw_fail(SWApplication *application, DWORD error);
void sw_enqueue(SWApplication *application, SWEvent *event);
SWEvent *sw_event(SWWindow *window, int32_t kind);
void sw_stamp_event(SWEventHeader *data);
void sw_emit(SWWindow *window, int32_t kind);
void sw_free_event(SWEvent *event);
void sw_watch_message(SWApplication *application, int64_t identity);
void sw_watch_retire(SWApplication *application);
LRESULT CALLBACK sw_window_proc(HWND hwnd, UINT message, WPARAM wparam,
                                LPARAM lparam);
SWSnapshot sw_snapshot(SWWindow *window);
DWORD sw_style(SWWindow *window);
int64_t sw_fit_fullscreen(SWWindow *window);
int64_t sw_restyle(SWWindow *window);
int64_t sw_frame_margins(SWWindow *window);
void sw_frame_invalidate(SWWindow *window);
void sw_frame_geometry(SWWindow *window, SWSnapshot snapshot);
void sw_frame_clear_buttons(SWWindow *window);
bool sw_frame_input(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
                    LRESULT *result);
void sw_adjust_frame_rect(SWWindow *window, RECT *rect, UINT dpi);
bool sw_frame_message(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
                      LRESULT *result);
int64_t sw_size(SWWindow *window, double width, double height);
void sw_minmax(SWWindow *window, MINMAXINFO *info);
void sw_sizing(SWWindow *window, WPARAM edge, RECT *rect);
bool sw_input(SWWindow *window, UINT message, WPARAM wparam, LPARAM lparam,
              LRESULT *result);
bool sw_menu_message(SWWindow *window, UINT message, WPARAM wparam,
                     LPARAM lparam, LRESULT *result);
bool sw_menu_translate(SWApplication *application, MSG *message);
void sw_menu_dpi(SWWindow *window, UINT dpi);
bool sw_text(SWWindow *window, UINT message, WPARAM wparam, LPARAM lparam,
             LRESULT *result);
bool sw_text_composing(SWWindow *window);
void sw_text_focus(SWWindow *window, bool focused);
void sw_text_geometry_changed(SWWindow *window);
void sw_text_window_destroy(SWWindow *window);
void sw_text_application_destroy(SWApplication *application);
void sw_input_init(SWWindow *window);
void sw_input_destroy(SWWindow *window);
void sw_drag_promoted_mouse(SWWindow *window, UINT message, WPARAM keys);
void sw_drag_contact_changed(SWWindow *window);
void sw_drag_window_destroy(SWWindow *window);
void sw_drop_window_destroy(SWWindow *window);
bool sw_drop_message(SWWindow *window, UINT message, WPARAM key);
int64_t sw_keyboard_start(SWWindow *window);
void sw_keyboard_geometry(SWWindow *window);
void sw_keyboard_destroy(SWWindow *window);
void sw_key(SWWindow *window, UINT message, WPARAM vk, LPARAM lp);
void sw_update_modifiers(SWWindow *window, uint64_t value);
bool sw_pointer_message(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
                        LRESULT *result);
void sw_reset_input(SWWindow *window);
void sw_cancel_pointer(SWWindow *window, int reason);
void sw_contact_message(SWWindow *window, UINT message, WPARAM wp);
void sw_cancel_contacts(SWWindow *window, int reason);
bool sw_compat_mouse_message(UINT message);
bool sw_modal_input_message(UINT message);
void sw_cancel_composition(SWWindow *window);
uint64_t sw_modifiers(void);
WCHAR *sw_wide(const uint8_t *text, int64_t length);
uint8_t *sw_utf8(const WCHAR *text, int length, int64_t *bytes);
double sw_now(void);
int sw_theme(SWWindow *window);

#ifdef __cplusplus
}
#endif

#endif
