#include "x11.h"
#include <xcb-imdkit/encoding.h>

static SWWindow *find_ic(SWApplication *app, xcb_xic_t ic) {
  for (SWWindow *window = app->windows; window; window = window->next)
    if (((SXWindow *)window->backend)->ic == ic)
      return window->ime_allowed ? window : NULL;
  return NULL;
}

static char *decode(xcb_xim_t *im, const char *text, size_t length,
                    size_t *out_length) {
  if (xcb_xim_get_encoding(im) == XCB_XIM_UTF8_STRING) {
    if (!sw_utf8_valid((const uint8_t *)text, length))
      return NULL;
    *out_length = length;
    return strndup(text, length);
  }
  return xcb_compound_text_to_utf8(text, length, out_length);
}

static void forward(xcb_xim_t *im, xcb_xic_t ic, xcb_key_press_event_t *event,
                    void *context) {
  SWWindow *window = find_ic(context, ic);
  if (!window || event->detail < 8)
    return;
  uint32_t code = event->detail - 8;
  bool down = (event->response_type & 127) == XCB_KEY_PRESS;
  sw_key(window, code, down, down && window->key_down[code], true);
}

static void commit(xcb_xim_t *im, xcb_xic_t ic, uint32_t flag, char *text,
                   uint32_t length, uint32_t *symbols, size_t count,
                   void *context) {
  SWWindow *window = find_ic(context, ic);
  if (!window)
    return;
  size_t size = 0;
  char *decoded = decode(im, text, length, &size);
  if (decoded) {
    sw_commit(window, decoded, size);
    free(decoded);
  } else
    sw_cancel_composition(window);
}

static void preedit_draw(xcb_xim_t *im, xcb_xic_t ic,
                         xcb_im_preedit_draw_fr_t *frame, void *context) {
  SWWindow *window = find_ic(context, ic);
  if (!window)
    return;
  const char *old = window->preedit ? window->preedit : "";
  size_t old_size = strlen(old), inserted_size = 0;
  int64_t start = sw_utf8_offset(old, old_size, frame->chg_first);
  int64_t end = sw_utf8_offset(old, old_size,
                               (uint64_t)frame->chg_first + frame->chg_length);
  char *inserted = frame->length_of_preedit_string
                       ? decode(im, (const char *)frame->preedit_string,
                                frame->length_of_preedit_string, &inserted_size)
                       : strdup("");
  if (start < 0 || end < start || !inserted ||
      inserted_size > SIZE_MAX - old_size - 1) {
    free(inserted);
    sw_cancel_composition(window);
    return;
  }
  size_t size = old_size - (size_t)(end - start) + inserted_size;
  char *text = malloc(size + 1);
  if (!text) {
    free(inserted);
    sw_fail(window->application, ENOMEM);
    return;
  }
  memcpy(text, old, start);
  memcpy(text + start, inserted, inserted_size);
  memcpy(text + start + inserted_size, old + end, old_size - end);
  text[size] = 0;
  int64_t caret = sw_utf8_offset(text, size, frame->caret);
  if (caret < 0)
    sw_cancel_composition(window);
  else
    sw_preedit(window, text, size, caret, caret);
  free(inserted);
  free(text);
}

static void preedit_caret(xcb_xim_t *im, xcb_xic_t ic,
                          xcb_im_preedit_caret_fr_t *frame, void *context) {
  SWWindow *window = find_ic(context, ic);
  if (!window || !window->preedit)
    return;
  size_t length = strlen(window->preedit);
  int64_t caret = sw_utf8_offset(window->preedit, length, frame->position);
  if (caret < 0)
    sw_cancel_composition(window);
  else
    sw_preedit(window, window->preedit, length, caret, caret);
}

static void preedit_done(xcb_xim_t *im, xcb_xic_t ic, void *context) {
  SWWindow *window = find_ic(context, ic);
  if (window)
    sw_cancel_composition(window);
}

static void ic_created(xcb_xim_t *im, xcb_xic_t ic, void *context) {
  SWApplication *app = sw_application;
  if (!app)
    return;
  int64_t identity = (intptr_t)context;
  for (SWWindow *window = app->windows; window; window = window->next) {
    if (window->identity != identity)
      continue;
    SXWindow *w = window->backend;
    w->ic_pending = false;
    if (window->ime_allowed) {
      w->ic = ic;
      sw_x11_ime(window);
      sw_x11_caret(window);
      return;
    }
  }
  if (ic)
    xcb_xim_destroy_ic(im, ic, NULL, NULL);
}

void sx_create_ic(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (!x->im_ready || !window->ime_allowed || w->ic || w->ic_pending)
    return;
  uint32_t style = XCB_IM_PreeditCallbacks | XCB_IM_StatusNothing;
  w->ic_pending =
      xcb_xim_create_ic(x->im, ic_created, (void *)(intptr_t)window->identity,
                        XCB_XIM_XNInputStyle, &style, XCB_XIM_XNClientWindow,
                        &w->window, XCB_XIM_XNFocusWindow, &w->window, NULL);
}

void sx_destroy_ic(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (w->ic && x->im) {
    xcb_xim_unset_ic_focus(x->im, w->ic);
    xcb_xim_destroy_ic(x->im, w->ic, NULL, NULL);
    w->ic = 0;
  }
}

static void opened(xcb_xim_t *im, void *context) {
  SWApplication *app = context;
  ((SXApplication *)app->backend)->im_ready = true;
  for (SWWindow *window = app->windows; window; window = window->next)
    sx_create_ic(window);
}

static void disconnected(xcb_xim_t *im, void *context) {
  SWApplication *app = context;
  ((SXApplication *)app->backend)->im_ready = false;
  for (SWWindow *window = app->windows; window; window = window->next) {
    SXWindow *w = window->backend;
    w->ic = 0;
    w->ic_pending = false;
    sw_cancel_composition(window);
  }
}

void sx_initialize_ime(SWApplication *app) {
  SXApplication *x = app->backend;
  xcb_compound_text_init();
  x->im = xcb_xim_create(x->connection, x->screen_number, NULL);
  if (!x->im)
    return;
  xcb_xim_set_use_utf8_string(x->im, true);
  xcb_xim_set_use_compound_text(x->im, true);
  static const xcb_xim_im_callback callbacks = {.forward_event = forward,
                                                .commit_string = commit,
                                                .preedit_draw = preedit_draw,
                                                .preedit_caret = preedit_caret,
                                                .preedit_done = preedit_done,
                                                .disconnected = disconnected};
  xcb_xim_set_im_callback(x->im, &callbacks, app);
  xcb_xim_open(x->im, opened, true, app);
}

void sw_x11_ime(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (!window->ime_allowed) {
    sx_destroy_ic(window);
    return;
  }
  sx_create_ic(window);
  if (w->ic) {
    if (window->snapshot.focused)
      xcb_xim_set_ic_focus(x->im, w->ic);
    else {
      xcb_xim_unset_ic_focus(x->im, w->ic);
      xcb_xim_reset_ic(x->im, w->ic, NULL, NULL);
    }
  }
}

void sw_x11_caret(SWWindow *window) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (!w->ic || !window->caret_known)
    return;
  double px = window->caret_x * window->snapshot.scale;
  double py = (window->caret_y + window->caret_height) * window->snapshot.scale;
  xcb_point_t point = {(int16_t)fmax(INT16_MIN, fmin(INT16_MAX, px)),
                       (int16_t)fmax(INT16_MIN, fmin(INT16_MAX, py))};
  xcb_xim_nested_list attributes =
      xcb_xim_create_nested_list(x->im, XCB_XIM_XNSpotLocation, &point, NULL);
  xcb_xim_set_ic_values(x->im, w->ic, NULL, NULL, XCB_XIM_XNPreeditAttributes,
                        &attributes, NULL);
  free(attributes.data);
}

bool sx_ime_key(SWWindow *window, xcb_key_press_event_t *event) {
  SXApplication *x = window->application->backend;
  SXWindow *w = window->backend;
  if (!window->ime_allowed || !w->ic ||
      (event->state & (XCB_MOD_MASK_CONTROL | XCB_MOD_MASK_1 | XCB_MOD_MASK_4)))
    return false;
  return xcb_xim_forward_event(x->im, w->ic, event);
}
