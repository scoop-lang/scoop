#include "wayland.h"

void sl_reset_text(SWApplication *app) {
  SLApplication *w = app->backend;
  free(w->preedit);
  free(w->commit);
  w->preedit = w->commit = NULL;
  w->preedit_set = w->commit_set = w->delete_set = false;
  w->delete_before = w->delete_after = 0;
}

void sl_text_commit(SWApplication *app) {
  SLApplication *w = app->backend;
  ++w->text_serial;
  int64_t window = w->text_window ? w->text_window->identity : 0;
  SLTextState *state = w->text_tail;
  if (!state || state->window != window ||
      state->connection != w->text_connection ||
      state->revision != w->text_revision ||
      state->surrounding != w->text_surrounding) {
    state = calloc(1, sizeof(*state));
    if (!state) {
      sw_fail(app, ENOMEM);
      return;
    }
    state->first = w->text_serial;
    state->window = window;
    state->connection = w->text_connection;
    state->revision = w->text_revision;
    state->surrounding = w->text_surrounding;
    if (w->text_tail)
      w->text_tail->next = state;
    else
      w->text_states = state;
    w->text_tail = state;
  }
  state->last = w->text_serial;
  zwp_text_input_v3_commit(w->text_input);
}

static void entered(void *data, struct zwp_text_input_v3 *input,
                    struct wl_surface *surface) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  w->text_window = sl_find(app, surface);
  w->text_enabled = false;
  w->text_blocked = w->text_pending = false;
  sl_reset_text(app);
  if (w->text_window)
    sw_wayland_ime(w->text_window);
}

static void left(void *data, struct zwp_text_input_v3 *input,
                 struct wl_surface *surface) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  if (w->text_window)
    sw_cancel_composition(w->text_window);
  w->text_window = NULL;
  w->text_enabled = false;
  w->text_connection = 0;
  sl_reset_text(app);
}

static void preedit(void *data, struct zwp_text_input_v3 *input,
                    const char *text, int32_t begin, int32_t end) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  char *copy = strdup(text ? text : "");
  if (!copy) {
    sw_fail(app, ENOMEM);
    return;
  }
  free(w->preedit);
  w->preedit = copy;
  w->preedit_begin = begin;
  w->preedit_end = end;
  w->preedit_set = true;
}
static void commit(void *data, struct zwp_text_input_v3 *input,
                   const char *text) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  char *copy = strdup(text ? text : "");
  if (!copy) {
    sw_fail(app, ENOMEM);
    return;
  }
  free(w->commit);
  w->commit = copy;
  w->commit_set = true;
}
static void delete_surrounding(void *data, struct zwp_text_input_v3 *input,
                               uint32_t before, uint32_t after) {
  SLApplication *w = ((SWApplication *)data)->backend;
  w->delete_before = before;
  w->delete_after = after;
  w->delete_set = true;
}

static SLTextState *find_state(SLApplication *w, uint32_t serial) {
  for (SLTextState *state = w->text_states; state; state = state->next)
    if ((uint32_t)(serial - state->first) <=
        (uint32_t)(state->last - state->first))
      return state;
  return NULL;
}

static void done(void *data, struct zwp_text_input_v3 *input, uint32_t serial) {
  SWApplication *app = data;
  SLApplication *w = app->backend;
  SWWindow *window = w->text_window;
  w->text_blocked = serial != w->text_serial;
  SLTextState *state = find_state(w, serial);
  if (!window || !window->ime_allowed || !state ||
      state->window != window->identity ||
      state->connection != window->connection) {
    sl_reset_text(app);
    return;
  }
  SWEvent *event = sw_text_batch(window);
  bool valid = event != NULL;
  if (valid && window->preedit_active)
    valid = sw_text_append(event, 0, "", 0, 0, 0);
  if (valid && w->delete_set && (w->delete_before || w->delete_after)) {
    valid = state->surrounding && state->revision >= 0;
    if (valid) {
      valid = sw_text_append(event, 3, "", 0, -1, -1);
      if (valid) {
        event->edits[event->edit_count - 1].anchor = w->delete_before;
        event->edits[event->edit_count - 1].cursor = w->delete_after;
        event->data.flags |= 1;
        event->data.b = state->revision;
      }
    }
  }
  if (valid && w->commit_set)
    valid = sw_text_append(event, 1, w->commit, strlen(w->commit), -1, -1);
  if (valid && w->preedit_set && *w->preedit) {
    int64_t anchor =
        w->preedit_begin < 0 || w->preedit_end < 0 ? -1 : w->preedit_begin;
    int64_t cursor = anchor < 0 ? -1 : w->preedit_end;
    valid = sw_text_append(event, 0, w->preedit, strlen(w->preedit), anchor,
                           cursor);
  }
  if (valid) {
    window->preedit_active = w->preedit_set && *w->preedit;
    free(window->preedit);
    window->preedit = w->preedit;
    w->preedit = NULL;
    sw_text_deliver(window, event);
  } else {
    sw_free_event(event);
    sw_cancel_composition(window);
  }
  // Earlier protocol revisions cannot be referenced by a subsequent ordered
  // done event.
  while (w->text_states && w->text_states != state) {
    SLTextState *old = w->text_states;
    w->text_states = old->next;
    free(old);
  }
  sl_reset_text(app);
  if (!w->text_blocked && w->text_pending)
    sw_wayland_document(window);
}
static const struct zwp_text_input_v3_listener listener = {
    .enter = entered,
    .leave = left,
    .preedit_string = preedit,
    .commit_string = commit,
    .delete_surrounding_text = delete_surrounding,
    .done = done};

void sl_setup_text(SWApplication *app) {
  SLApplication *w = app->backend;
  if (!w->text_input && w->text_manager && w->seat) {
    w->text_input =
        zwp_text_input_manager_v3_get_text_input(w->text_manager, w->seat);
    zwp_text_input_v3_add_listener(w->text_input, &listener, app);
  }
}

bool sl_text_active(SWWindow *window) {
  SLApplication *w = window->application->backend;
  return w->text_input && w->text_window == window && w->text_enabled;
}

static void send_caret(SWWindow *window) {
  SLApplication *w = window->application->backend;
  if (!window->caret_known)
    return;
  double x = floor(window->caret_x), y = floor(window->caret_y);
  double width = ceil(window->caret_width), height = ceil(window->caret_height);
  if (x < INT_MIN || x > INT_MAX || y < INT_MIN || y > INT_MAX ||
      width > INT_MAX || height > INT_MAX)
    return;
  zwp_text_input_v3_set_cursor_rectangle(w->text_input, x, y, width, height);
}

static bool send_surrounding(SWWindow *window) {
  SLApplication *w = window->application->backend;
  SWTextSnapshotData *doc = &window->document;
  if (doc->revision < 0 || window->applied_sequence != window->sequence)
    return false;
  int64_t removed =
      doc->marked_start >= 0 ? doc->marked_end - doc->marked_start : 0;
  int64_t length = doc->length - removed;
  int64_t anchor = doc->anchor, cursor = doc->cursor;
  if (removed) {
    anchor = anchor >= doc->marked_end    ? anchor - removed
             : anchor > doc->marked_start ? doc->marked_start
                                          : anchor;
    cursor = cursor >= doc->marked_end    ? cursor - removed
             : cursor > doc->marked_start ? doc->marked_start
                                          : cursor;
  }
  int64_t low = anchor < cursor ? anchor : cursor;
  int64_t high = anchor > cursor ? anchor : cursor;
  if (high - low > 4000)
    return false;
  int64_t start = low > 2000 ? low - 2000 : 0;
  int64_t end = start + 4000 < length ? start + 4000 : length;
  if (end < high) {
    end = high;
    start = end - 4000;
  }
  // Only the protocol excerpt is copied; large documents retain their normal
  // Scoop storage.
  char text[4001];
  size_t count = 0;
  for (int64_t index = start; index < end; ++index) {
    int64_t original =
        removed && index >= doc->marked_start ? index + removed : index;
    text[count++] = doc->text[original];
  }
  while (count && ((uint8_t)text[0] & 0xc0) == 0x80) {
    memmove(text, text + 1, --count);
    ++start;
  }
  while (count && !sw_utf8_valid((uint8_t *)text, count))
    --count;
  if (low < start || high > start + (int64_t)count || memchr(text, 0, count))
    return false;
  text[count] = 0;
  zwp_text_input_v3_set_surrounding_text(w->text_input, text, cursor - start,
                                         anchor - start);
  zwp_text_input_v3_set_text_change_cause(
      w->text_input, doc->after ? ZWP_TEXT_INPUT_V3_CHANGE_CAUSE_INPUT_METHOD
                                : ZWP_TEXT_INPUT_V3_CHANGE_CAUSE_OTHER);
  w->text_revision = doc->revision;
  return true;
}

void sw_wayland_ime(SWWindow *window) {
  SLApplication *w = window->application->backend;
  if (!w->text_input || w->text_window != window)
    return;
  bool enabled = window->ime_allowed;
  if (enabled == w->text_enabled &&
      (!enabled || w->text_connection == window->connection))
    return;
  sl_reset_text(window->application);
  w->text_enabled = enabled;
  w->text_blocked = w->text_pending = false;
  w->text_connection = enabled ? window->connection : 0;
  w->text_revision = -1;
  w->text_surrounding = false;
  if (enabled) {
    zwp_text_input_v3_enable(w->text_input);
    zwp_text_input_v3_set_content_type(
        w->text_input, ZWP_TEXT_INPUT_V3_CONTENT_HINT_NONE,
        ZWP_TEXT_INPUT_V3_CONTENT_PURPOSE_NORMAL);
    w->text_surrounding = send_surrounding(window);
    send_caret(window);
  } else
    zwp_text_input_v3_disable(w->text_input);
  sl_text_commit(window->application);
}

void sw_wayland_document(SWWindow *window) {
  SLApplication *w = window->application->backend;
  if (!sl_text_active(window))
    return;
  w->text_pending = true;
  if (w->text_blocked || window->applied_sequence != window->sequence)
    return;
  w->text_pending = false;
  bool supplied = send_surrounding(window);
  if (!supplied && w->text_surrounding) {
    // Reset unsupported surrounding data instead of leaving an older document
    // available.
    zwp_text_input_v3_disable(w->text_input);
    zwp_text_input_v3_enable(w->text_input);
    w->text_revision = -1;
  }
  w->text_surrounding = supplied;
  send_caret(window);
  sl_text_commit(window->application);
}
void sw_wayland_caret(SWWindow *window) {
  SLApplication *w = window->application->backend;
  if (!sl_text_active(window))
    return;
  if (w->text_blocked || window->applied_sequence != window->sequence) {
    w->text_pending = true;
    return;
  }
  send_caret(window);
  sl_text_commit(window->application);
}
