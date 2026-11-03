#include "wayland.h"
#include <linux/input-event-codes.h>
#include <sys/mman.h>
#include <sys/stat.h>

static void keymap(void *data, struct wl_keyboard *keyboard, uint32_t format,
                   int32_t fd, uint32_t size) {
  SWApplication *app = data;
  struct stat info;
  if (format != WL_KEYBOARD_KEYMAP_FORMAT_XKB_V1 || !size || fstat(fd, &info) ||
      info.st_size < size) {
    close(fd);
    sw_fail(app, EPROTO);
    return;
  }
  char *text = mmap(NULL, size, PROT_READ, MAP_PRIVATE, fd, 0);
  close(fd);
  if (text == MAP_FAILED) {
    sw_fail(app, errno);
    return;
  }
  struct xkb_keymap *map = text[size - 1] == 0
                               ? xkb_keymap_new_from_string(
                                     app->xkb, text, XKB_KEYMAP_FORMAT_TEXT_V1,
                                     XKB_KEYMAP_COMPILE_NO_FLAGS)
                               : NULL;
  munmap(text, size);
  struct xkb_state *state = map ? xkb_state_new(map) : NULL;
  if (!state) {
    xkb_keymap_unref(map);
    sw_fail(app, EPROTO);
    return;
  }
  xkb_state_unref(app->key_state);
  xkb_keymap_unref(app->keymap);
  app->keymap = map;
  app->key_state = state;
}

static void keyboard_enter(void *data, struct wl_keyboard *keyboard,
                           uint32_t serial, struct wl_surface *surface,
                           struct wl_array *keys) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  SWWindow *window = sl_find(app, surface);
  if (w->keyboard_window && w->keyboard_window != window)
    sw_focus(w->keyboard_window, false);
  w->keyboard_window = window;
  w->repeat_deadline = -1;
  memset(app->physical_keys, 0, sizeof(app->physical_keys));
  app->physical_keys_known = true;
  uint32_t *key;
  wl_array_for_each(key, keys) if (*key < 768) app->physical_keys[*key] = true;
  if (window)
    sw_focus(window, true);
}
static void keyboard_leave(void *data, struct wl_keyboard *keyboard,
                           uint32_t serial, struct wl_surface *surface) {
  SLApplication *w = ((SWApplication *)data)->backend;
  if (w->keyboard_window)
    sw_focus(w->keyboard_window, false);
  w->keyboard_window = NULL;
  w->repeat_deadline = -1;
}
static void keyboard_key(void *data, struct wl_keyboard *keyboard,
                         uint32_t serial, uint32_t time, uint32_t key,
                         uint32_t state) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  SWWindow *window = w->keyboard_window;
  if (!window || !app->key_state)
    return;
  w->serial = serial;
  bool down = state == WL_KEYBOARD_KEY_STATE_PRESSED;
  bool repeat = state == WL_KEYBOARD_KEY_STATE_REPEATED;
  if (key < 768)
    app->physical_keys[key] = down || repeat;
  // An input method forwards only keys it did not consume. Ordinary text must
  // still work when the compositor exposes text-input but has no active IME.
  sw_key(window, key, down || repeat, repeat, true);
  if (down && w->repeat_rate > 0 &&
      xkb_keymap_key_repeats(app->keymap, key + 8)) {
    w->repeat_key = key;
    w->repeat_deadline = sw_now() + w->repeat_delay / 1000.0;
  } else if (!down && key == w->repeat_key)
    w->repeat_deadline = -1;
}
static void keyboard_modifiers(void *data, struct wl_keyboard *keyboard,
                               uint32_t serial, uint32_t depressed,
                               uint32_t latched, uint32_t locked,
                               uint32_t group) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  if (app->key_state)
    xkb_state_update_mask(app->key_state, depressed, latched, locked, 0, 0,
                          group);
  if (w->keyboard_window)
    sw_update_modifiers(w->keyboard_window);
}
static void repeat_info(void *data, struct wl_keyboard *keyboard, int32_t rate,
                        int32_t delay) {
  SLApplication *w = ((SWApplication *)data)->backend;
  w->repeat_rate = rate > 0 ? rate : 0;
  w->repeat_delay = delay > 0 ? delay : 0;
  if (!w->repeat_rate)
    w->repeat_deadline = -1;
}
static const struct wl_keyboard_listener keyboard_listener = {
    .keymap = keymap,
    .enter = keyboard_enter,
    .leave = keyboard_leave,
    .key = keyboard_key,
    .modifiers = keyboard_modifiers,
    .repeat_info = repeat_info};

static void pointer_enter(void *data, struct wl_pointer *pointer,
                          uint32_t serial, struct wl_surface *surface,
                          wl_fixed_t x, wl_fixed_t y) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  w->pointer_serial = serial;
  w->pointer_window = sl_find(app, surface);
  if (w->pointer_window) {
    SWWindow *window = w->pointer_window;
    window->pointer_x = wl_fixed_to_double(x);
    window->pointer_y = wl_fixed_to_double(y);
    sw_pointer(window, 0, 0, 0, 0, 0);
    sw_wayland_cursor(window);
  }
}
static void pointer_leave(void *data, struct wl_pointer *pointer,
                          uint32_t serial, struct wl_surface *surface) {
  SLApplication *w = ((SWApplication *)data)->backend;
  if (w->pointer_window)
    sw_pointer(w->pointer_window, 1, 0, 0, 0, 0);
  w->pointer_window = NULL;
  w->pointer_serial = 0;
}
static void pointer_motion(void *data, struct wl_pointer *pointer,
                           uint32_t time, wl_fixed_t x, wl_fixed_t y) {
  SLApplication *w = ((SWApplication *)data)->backend;
  if (w->pointer_window) {
    w->pointer_window->pointer_x = wl_fixed_to_double(x);
    w->pointer_window->pointer_y = wl_fixed_to_double(y);
    sw_pointer(w->pointer_window, 2, 0, 0, 0, 0);
  }
}
static void pointer_button(void *data, struct wl_pointer *pointer,
                           uint32_t serial, uint32_t time, uint32_t button,
                           uint32_t state) {
  SLApplication *w = ((SWApplication *)data)->backend;
  w->serial = serial;
  if (!w->pointer_window)
    return;
  int64_t mapped = button == BTN_LEFT     ? 0
                   : button == BTN_RIGHT  ? 1
                   : button == BTN_MIDDLE ? 2
                   : button >= BTN_MOUSE  ? button - BTN_MOUSE
                                          : button;
  sw_pointer(w->pointer_window,
             state == WL_POINTER_BUTTON_STATE_PRESSED ? 3 : 4, mapped, 0, 0, 0);
}
static void pointer_frame(void *data, struct wl_pointer *pointer) {
  SLApplication *w = ((SWApplication *)data)->backend;
  SWWindow *window = w->pointer_window;
  if (window &&
      (w->axis_x || w->axis_y || w->lines_x || w->lines_y || w->axis_stop)) {
    int phase = 0;
    if (w->axis_source == WL_POINTER_AXIS_SOURCE_FINGER) {
      phase = w->axis_stop ? 8 : w->scroll_active ? 4 : 1;
      w->scroll_active = !w->axis_stop;
    }
    sw_pointer(
        window, w->axis_discrete ? 5 : 6, 0,
        w->axis_discrete ? -w->lines_x : -w->axis_x * window->snapshot.scale,
        w->axis_discrete ? -w->lines_y : -w->axis_y * window->snapshot.scale,
        phase);
  }
  w->axis_x = w->axis_y = w->lines_x = w->lines_y = 0;
  w->axis_discrete = w->axis_stop = false;
}
static void pointer_axis(void *data, struct wl_pointer *pointer, uint32_t time,
                         uint32_t axis, wl_fixed_t value) {
  SLApplication *w = ((SWApplication *)data)->backend;
  if (axis == WL_POINTER_AXIS_HORIZONTAL_SCROLL)
    w->axis_x += wl_fixed_to_double(value);
  else
    w->axis_y += wl_fixed_to_double(value);
  if (wl_pointer_get_version(pointer) < WL_POINTER_FRAME_SINCE_VERSION)
    pointer_frame(data, pointer);
}
static void axis_source(void *data, struct wl_pointer *pointer,
                        uint32_t source) {
  ((SLApplication *)((SWApplication *)data)->backend)->axis_source = source;
}
static void axis_stop(void *data, struct wl_pointer *pointer, uint32_t time,
                      uint32_t axis) {
  ((SLApplication *)((SWApplication *)data)->backend)->axis_stop = true;
}
static void axis_discrete(void *data, struct wl_pointer *pointer, uint32_t axis,
                          int32_t steps) {
  SLApplication *w = ((SWApplication *)data)->backend;
  w->axis_discrete = true;
  if (axis == WL_POINTER_AXIS_HORIZONTAL_SCROLL)
    w->lines_x += steps;
  else
    w->lines_y += steps;
}
static void axis_value120(void *data, struct wl_pointer *pointer, uint32_t axis,
                          int32_t value) {
  SLApplication *w = ((SWApplication *)data)->backend;
  w->axis_discrete = true;
  if (axis == WL_POINTER_AXIS_HORIZONTAL_SCROLL)
    w->lines_x = value / 120.0;
  else
    w->lines_y = value / 120.0;
}
static const struct wl_pointer_listener pointer_listener = {
    .enter = pointer_enter,
    .leave = pointer_leave,
    .motion = pointer_motion,
    .button = pointer_button,
    .axis = pointer_axis,
    .frame = pointer_frame,
    .axis_source = axis_source,
    .axis_stop = axis_stop,
    .axis_discrete = axis_discrete,
    .axis_value120 = axis_value120};

void sl_seat_capabilities(void *data, struct wl_seat *seat,
                          uint32_t capabilities) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  if ((capabilities & WL_SEAT_CAPABILITY_KEYBOARD) && !w->keyboard) {
    w->keyboard = wl_seat_get_keyboard(seat);
    wl_keyboard_add_listener(w->keyboard, &keyboard_listener, app);
  } else if (!(capabilities & WL_SEAT_CAPABILITY_KEYBOARD) && w->keyboard) {
    if (w->keyboard_window)
      sw_focus(w->keyboard_window, false);
    w->keyboard_window = NULL;
    w->repeat_deadline = -1;
    wl_keyboard_release(w->keyboard);
    w->keyboard = NULL;
  }
  if ((capabilities & WL_SEAT_CAPABILITY_POINTER) && !w->pointer) {
    w->pointer = wl_seat_get_pointer(seat);
    wl_pointer_add_listener(w->pointer, &pointer_listener, app);
  } else if (!(capabilities & WL_SEAT_CAPABILITY_POINTER) && w->pointer) {
    if (w->pointer_window) {
      sw_cancel_pointer(w->pointer_window, 1);
      sw_pointer(w->pointer_window, 1, 0, 0, 0, 0);
    }
    w->pointer_window = NULL;
    wl_pointer_release(w->pointer);
    w->pointer = NULL;
  }
  if (w->decor) {
    sl_setup_clipboard(app);
    sl_setup_text(app);
  }
}

int64_t sw_wayland_cursor(SWWindow *window) {
  SLApplication *w = window->application->backend;
  if (!w->pointer || w->pointer_window != window || !w->pointer_serial)
    return 0;
  if (!window->cursor_visible) {
    wl_pointer_set_cursor(w->pointer, w->pointer_serial, NULL, 0, 0);
    return 0;
  }
  int scale = (int)ceil(window->snapshot.scale);
  if (scale > INT_MAX / 24)
    return EOVERFLOW;
  if (!w->cursor_theme || w->cursor_scale != scale) {
    struct wl_cursor_theme *theme =
        wl_cursor_theme_load(NULL, 24 * scale, w->shm);
    if (!theme)
      return ENOMEM;
    if (w->cursor_theme)
      wl_cursor_theme_destroy(w->cursor_theme);
    w->cursor_theme = theme;
    w->cursor_scale = scale;
  }
  const char *names[] = {"default",     "text",      "crosshair",
                         "pointer",     "grab",      "grabbing",
                         "not-allowed", "ew-resize", "ns-resize"};
  struct wl_cursor *cursor =
      wl_cursor_theme_get_cursor(w->cursor_theme, names[window->cursor_icon]);
  if (!cursor)
    cursor = wl_cursor_theme_get_cursor(w->cursor_theme, "left_ptr");
  if (!cursor || !cursor->image_count)
    return -2;
  struct wl_cursor_image *image = cursor->images[0];
  struct wl_buffer *buffer = wl_cursor_image_get_buffer(image);
  if (!buffer)
    return ENOMEM;
  wl_surface_set_buffer_scale(w->cursor_surface, scale);
  wl_surface_attach(w->cursor_surface, buffer, 0, 0);
  wl_surface_damage(w->cursor_surface, 0, 0, INT_MAX, INT_MAX);
  wl_surface_commit(w->cursor_surface);
  wl_pointer_set_cursor(w->pointer, w->pointer_serial, w->cursor_surface,
                        image->hotspot_x / scale, image->hotspot_y / scale);
  return 0;
}

double sw_wayland_repeat_deadline(SWApplication *app) {
  return ((SLApplication *)app->backend)->repeat_deadline;
}
void sw_wayland_repeat(SWApplication *app) {
  SLApplication *w = app->backend;
  if (w->repeat_deadline < 0 || sw_now() < w->repeat_deadline ||
      !w->keyboard_window)
    return;
  sw_key(w->keyboard_window, w->repeat_key, true, true,
         !sl_text_active(w->keyboard_window));
  w->repeat_deadline =
      w->repeat_rate > 0 ? sw_now() + 1.0 / w->repeat_rate : -1;
}
