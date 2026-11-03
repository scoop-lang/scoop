#ifndef SCOOP_WINDOW_X11_H
#define SCOOP_WINDOW_X11_H

#include "window.h"
#include <xcb-imdkit/imclient.h>
#include <xcb/randr.h>
#include <xcb/xcb_cursor.h>
#include <xcb/xkb.h>
#include <xkbcommon/xkbcommon-x11.h>

typedef struct SXTransfer {
  xcb_window_t window;
  xcb_atom_t property;
  xcb_atom_t target;
  uint8_t *text;
  size_t length, offset;
  struct SXTransfer *next;
} SXTransfer;

typedef struct {
  xcb_connection_t *connection;
  xcb_screen_t *screen;
  int screen_number;
  uint8_t randr_event, xkb_event;
  int keyboard_device;
  double scale;
  xcb_cursor_context_t *cursors;
  xcb_xim_t *im;
  bool im_ready;
  xcb_window_t helper;
  xcb_atom_t protocols, delete_window, take_focus, utf8, name, pid, wm_state;
  xcb_atom_t net_state, above, below, maximized_h, maximized_v, hidden,
      attention;
  xcb_atom_t active, moveresize, frame_extents, motif, normal_hints, size_hints;
  xcb_atom_t wm_hints, theme, clipboard, targets, text, transfer, incr,
      timestamp;
  uint8_t *clipboard_text;
  size_t clipboard_length;
  bool owns_clipboard, reading, read_done, read_incr;
  xcb_atom_t read_target;
  SWClipboardText incoming;
  SXTransfer *transfers;
} SXApplication;

typedef struct {
  xcb_window_t window;
  xcb_visualid_t visual;
  uint8_t depth;
  xcb_colormap_t colormap;
  xcb_cursor_t cursor;
  xcb_xic_t ic;
  bool ic_pending, created, colormap_created;
} SXWindow;

int sx_check(SWApplication *app, xcb_void_cookie_t cookie);
SWWindow *sx_find(SWApplication *app, xcb_window_t window);
void sx_window_event(SWApplication *app, xcb_generic_event_t *event);
void sx_input_event(SWWindow *window, xcb_generic_event_t *event);
void sx_keyboard_refresh(SWApplication *app);
void sx_update_state(SWWindow *window);
void sx_update_position(SWWindow *window);
void sx_monitor_change(SWApplication *app);
void sx_initialize_ime(SWApplication *app);
void sx_create_ic(SWWindow *window);
void sx_destroy_ic(SWWindow *window);
bool sx_ime_key(SWWindow *window, xcb_key_press_event_t *event);
bool sx_clipboard_event(SWApplication *app, xcb_generic_event_t *event);
void sx_clipboard_finish(SWApplication *app);
int64_t sx_state_request(SWWindow *window, bool enabled, xcb_atom_t first,
                         xcb_atom_t second);

#endif
