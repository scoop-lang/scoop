#ifndef SCOOP_WINDOW_WAYLAND_H
#define SCOOP_WINDOW_WAYLAND_H

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include "protocol/fractional-scale-v1.h"
#include "protocol/text-input-unstable-v3.h"
#include "protocol/viewporter.h"
#include "protocol/xdg-activation-v1.h"
#include "window.h"
#include <libdecor-0/libdecor.h>
#include <wayland-client.h>
#include <wayland-cursor.h>

typedef struct SLOutput {
  struct wl_output *output;
  SWApplication *application;
  uint32_t name;
  int width, height, scale, refresh;
  char *description;
  struct SLOutput *next;
} SLOutput;
typedef struct SLEnteredOutput {
  SLOutput *output;
  struct SLEnteredOutput *next;
} SLEnteredOutput;
typedef struct SLOffer {
  struct wl_data_offer *offer;
  const char *mime;
  struct SLOffer *next;
} SLOffer;
typedef struct SLSource {
  struct wl_data_source *source;
  SWApplication *application;
  uint8_t *text;
  size_t length;
  bool cancelled;
  struct SLSource *next;
} SLSource;
typedef struct SLTransfer {
  int fd;
  uint8_t *text;
  size_t length, offset;
  struct SLTransfer *next;
} SLTransfer;
typedef struct SLTextState {
  uint32_t first, last;
  int64_t window, connection, revision;
  bool surrounding;
  struct SLTextState *next;
} SLTextState;
typedef struct SLActivation {
  SWApplication *application;
  int64_t identity;
  struct xdg_activation_token_v1 *token;
  struct SLActivation *next;
} SLActivation;
typedef struct {
  struct wl_display *display;
  struct wl_registry *registry;
  struct wl_compositor *compositor;
  struct wl_shm *shm;
  struct libdecor *decor;
  struct wl_seat *seat;
  struct wl_keyboard *keyboard;
  struct wl_pointer *pointer;
  struct wl_surface *cursor_surface;
  struct wl_cursor_theme *cursor_theme;
  int cursor_scale;
  struct wp_viewporter *viewporter;
  struct wp_fractional_scale_manager_v1 *fractional;
  struct xdg_activation_v1 *activation;
  struct wl_data_device_manager *data_manager;
  struct wl_data_device *data_device;
  struct zwp_text_input_manager_v3 *text_manager;
  struct zwp_text_input_v3 *text_input;
  uint32_t seat_name, serial, pointer_serial, text_serial;
  bool text_enabled, text_blocked, text_pending;
  SWWindow *keyboard_window, *pointer_window, *text_window;
  uint32_t repeat_key;
  int repeat_rate, repeat_delay;
  double repeat_deadline;
  double axis_x, axis_y, lines_x, lines_y;
  bool axis_discrete, axis_stop, scroll_active;
  uint32_t axis_source;
  bool preedit_set, commit_set, delete_set;
  char *preedit, *commit;
  int32_t preedit_begin, preedit_end;
  uint32_t delete_before, delete_after;
  int64_t text_revision, text_connection;
  bool text_surrounding;
  SLTextState *text_states, *text_tail;
  SLOutput *outputs;
  SLOffer *offers, *selection;
  SLSource *sources, *current_source;
  SLTransfer *transfers;
  SLActivation *activations;
} SLApplication;
typedef struct {
  struct wl_surface *surface;
  struct libdecor_frame *frame;
  struct wp_viewport *viewport;
  struct wp_fractional_scale_v1 *fractional;
  SLEnteredOutput *outputs;
  char *title;
  double fractional_scale;
  int preferred_scale;
  bool mapped;
} SLWindow;

void sl_seat_capabilities(void *context, struct wl_seat *seat,
                          uint32_t capabilities);
void sl_window_scale(SWWindow *window);
void sl_monitor_changed(SWApplication *app);
void sl_setup_clipboard(SWApplication *app);
void sl_finish_clipboard(SWApplication *app);
void sl_clipboard_flush(SWApplication *app);
void sl_setup_text(SWApplication *app);
void sl_reset_text(SWApplication *app);
void sl_text_commit(SWApplication *app);
bool sl_text_active(SWWindow *window);
SWWindow *sl_find(SWApplication *app, struct wl_surface *surface);

#endif
