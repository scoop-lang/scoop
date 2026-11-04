#include "text_internal.h"

#include <algorithm>
#include <limits>

static SWTextState *state_of(SWWindow *window) {
  return static_cast<SWTextState *>(window->text);
}

static SWTextManager *manager_of(SWApplication *application) {
  return static_cast<SWTextManager *>(application->text_manager);
}

static void close_text_store(SWTextState *state) {
  if (state->focused) {
    SWTextManager *manager = manager_of(state->window->application);
    if (manager)
      manager->thread->SetFocus(NULL);
    state->focused = false;
  }
  sw_text_store_close(state);
}

static bool marked(const SWTextDocument &document) {
  return document.marked_start >= 0;
}

static void clear_event(SWEvent *event) {
  if (!event)
    return;
  for (int64_t index = 0; index < event->edit_count; ++index)
    free((void *)event->edits[index].text);
  event->edit_count = 0;
}

static bool append(SWTextState *state, int32_t kind, const WCHAR *text,
                   int length, int64_t anchor = -1, int64_t cursor = -1,
                   bool explicit_range = false, LONG start = 0, LONG end = 0) {
  SWEvent *event = state->event;
  if (!event)
    return false;
  if (event->edit_count == event->edit_capacity) {
    int64_t capacity = event->edit_capacity ? event->edit_capacity * 2 : 4;
    if (capacity < event->edit_capacity ||
        (uint64_t)capacity > SIZE_MAX / sizeof(SWTextEditData))
      return false;
    void *edits =
        realloc(event->edits, (size_t)capacity * sizeof(SWTextEditData));
    if (!edits)
      return false;
    event->edits = static_cast<SWTextEditData *>(edits);
    event->edit_capacity = capacity;
  }
  SWTextEditData edit = {};
  edit.kind = kind;
  edit.anchor = anchor;
  edit.cursor = cursor;
  edit.text = sw_utf8(text, length, &edit.text_length);
  if (!edit.text)
    return false;
  if (explicit_range) {
    int64_t first = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS,
                                        state->projection.text.data(), start,
                                        NULL, 0, NULL, NULL);
    int64_t last = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS,
                                       state->projection.text.data(), end,
                                       NULL, 0, NULL, NULL);
    if ((start && !first) || (end && !last)) {
      free((void *)edit.text);
      return false;
    }
    edit.has_replacement = 1;
    edit.replacement_start = first;
    edit.replacement_end = last;
  }
  event->edits[event->edit_count++] = edit;
  return true;
}

bool sw_text_boundary(const std::wstring &text, LONG offset) {
  if (offset < 0 || (size_t)offset > text.size())
    return false;
  if (offset == 0 || (size_t)offset == text.size())
    return true;
  WCHAR previous = text[(size_t)offset - 1], next = text[(size_t)offset];
  return !(previous >= 0xd800 && previous <= 0xdbff && next >= 0xdc00 &&
           next <= 0xdfff);
}

bool sw_text_range(const std::wstring &text, LONG start, LONG end) {
  return end >= start && sw_text_boundary(text, start) &&
         sw_text_boundary(text, end);
}

static bool valid_text(const WCHAR *text, ULONG length) {
  if (!text && length)
    return false;
  for (ULONG index = 0; index < length; ++index) {
    WCHAR value = text[index];
    if (value >= 0xd800 && value <= 0xdbff) {
      if (++index == length || text[index] < 0xdc00 || text[index] > 0xdfff)
        return false;
    } else if (value >= 0xdc00 && value <= 0xdfff) {
      return false;
    }
  }
  return true;
}

const SWTextDocument *sw_text_query(SWTextState *state) {
  if (!state || state->closed)
    return nullptr;
  if (state->editing)
    return &state->projection;
  if (state->document.present)
    return state->applied == state->sequence ? &state->document : nullptr;
  return &state->local;
}

static void invalidate_caret(SWTextState *state) {
  state->caret_known = false;
  sw_text_notify_layout(state);
}

void sw_text_invalidate_caret(SWTextState *state) {
  if (state && !state->closed)
    invalidate_caret(state);
}

bool sw_text_begin_edit(SWTextState *state) {
  if (!state || state->closed || state->canceling)
    return false;
  if (state->editing)
    return true;
  if (state->document.present && state->applied != state->sequence)
    return false;
  if (state->sequence == INT64_MAX)
    return false;
  state->projection = state->document.present ? state->document : state->local;
  state->event = sw_event(state->window, 13);
  if (!state->event)
    return false;
  state->editing = true;
  state->edit_invalid = false;
  state->uses_range = false;
  state->had_marked = marked(state->projection);
  return true;
}

HRESULT sw_text_replace(SWTextState *state, LONG start, LONG end,
                        const WCHAR *text, ULONG length, bool explicit_range,
                        TS_TEXTCHANGE *change) {
  if (!state->editing || state->closed || state->canceling)
    return TF_E_NOLOCK;
  if (length > LONG_MAX || !valid_text(text, length) ||
      !sw_text_range(state->projection.text, start, end) ||
      (int64_t)start + length > LONG_MAX)
    return TF_E_INVALIDPOS;
  const WCHAR *insertion = text ? text : L"";
  int32_t kind = state->composing ? 0 : 1;
  bool empty_preedit = kind == 0 && length == 0 && !marked(state->projection);
  std::wstring updated;
  const WCHAR *emitted = insertion;
  ULONG emitted_length = length;
  LONG selection_offset = (LONG)length;
  if (kind == 0 && !state->projection.present) {
    updated = state->projection.text;
    updated.replace((size_t)start, (size_t)(end - start), insertion,
                    (size_t)length);
    if (updated.size() > LONG_MAX)
      return TF_E_INVALIDPOS;
    emitted = updated.data();
    emitted_length = (ULONG)updated.size();
    selection_offset = start + (LONG)length;
  }
  int64_t selection = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS,
                                          emitted, (int)selection_offset, NULL,
                                          0, NULL, NULL);
  if (selection_offset && !selection)
    return E_INVALIDARG;
  bool use_range = explicit_range && state->projection.present;
  if (!empty_preedit &&
      !append(state, kind, emitted, (int)emitted_length,
              kind == 0 ? selection : -1, kind == 0 ? selection : -1,
              use_range, start, end))
    return E_OUTOFMEMORY;
  state->uses_range = state->uses_range || use_range;
  state->projection.text.replace((size_t)start, (size_t)(end - start),
                                 insertion, (size_t)length);
  LONG next = start + (LONG)length;
  state->projection.anchor = state->projection.cursor = next;
  if (kind == 0) {
    state->projection.marked_start = start;
    state->projection.marked_end = next;
  } else {
    state->projection.marked_start = state->projection.marked_end = -1;
  }
  if (change) {
    change->acpStart = start;
    change->acpOldEnd = end;
    change->acpNewEnd = next;
  }
  return S_OK;
}

HRESULT sw_text_select(SWTextState *state,
                       const TS_SELECTION_ACP &selection) {
  if (!state->editing || !state->composing)
    return E_NOTIMPL;
  if (!marked(state->projection) ||
      selection.acpStart < state->projection.marked_start ||
      selection.acpEnd > state->projection.marked_end ||
      selection.acpStart > selection.acpEnd ||
      !sw_text_boundary(state->projection.text, selection.acpStart) ||
      !sw_text_boundary(state->projection.text, selection.acpEnd) ||
      (selection.style.ase != TS_AE_START && selection.style.ase != TS_AE_END))
    return TF_E_INVALIDPOS;
  state->projection.anchor = selection.style.ase == TS_AE_START
                                 ? selection.acpEnd
                                 : selection.acpStart;
  state->projection.cursor = selection.style.ase == TS_AE_START
                                 ? selection.acpStart
                                 : selection.acpEnd;
  if (state->event && state->event->edit_count) {
    SWTextEditData &edit = state->event->edits[state->event->edit_count - 1];
    if (edit.kind == 0) {
      LONG base = state->projection.marked_start;
      int64_t anchor = WideCharToMultiByte(
          CP_UTF8, WC_ERR_INVALID_CHARS, state->projection.text.data() + base,
          state->projection.anchor - base, NULL, 0, NULL, NULL);
      int64_t cursor = WideCharToMultiByte(
          CP_UTF8, WC_ERR_INVALID_CHARS, state->projection.text.data() + base,
          state->projection.cursor - base, NULL, 0, NULL, NULL);
      if ((state->projection.anchor != base && !anchor) ||
          (state->projection.cursor != base && !cursor))
        return E_INVALIDARG;
      edit.anchor = anchor;
      edit.cursor = cursor;
    }
  }
  return S_OK;
}

static void queue_batch(SWTextState *state) {
  SWEvent *event = state->event;
  state->event = nullptr;
  if (!event)
    return;
  if (!event->edit_count || state->closed) {
    sw_free_event(event);
    return;
  }
  event->data.a = state->connection;
  event->data.c = ++state->sequence;
  if (state->uses_range) {
    event->data.flags = 1;
    event->data.b = state->document.revision;
  }
  invalidate_caret(state);
  sw_enqueue(state->window->application, event);
}

void sw_text_end_edit(SWTextState *state, bool invalid) {
  if (!state || !state->editing)
    return;
  bool rejected = invalid || state->edit_invalid;
  if (rejected) {
    clear_event(state->event);
    state->uses_range = false;
    if (state->had_marked && !append(state, 0, L"", 0))
      sw_fail(state->window->application, ERROR_NOT_ENOUGH_MEMORY);
    state->composing = false;
    state->local.marked_start = state->local.marked_end = -1;
    if (!state->document.present)
      state->local = SWTextDocument{};
  } else if (!state->document.present && !marked(state->projection)) {
    state->local = SWTextDocument{};
  } else {
    state->local = state->projection;
  }
  state->editing = false;
  queue_batch(state);
  state->projection = SWTextDocument{};
  if (rejected)
    sw_text_store_terminate_composition(state);
}

static void standalone(SWTextState *state, int32_t kind) {
  if (!state || state->closed)
    return;
  if (state->sequence == INT64_MAX) {
    sw_fail(state->window->application, ERROR_ARITHMETIC_OVERFLOW);
    return;
  }
  state->event = sw_event(state->window, 13);
  state->uses_range = false;
  if (!state->event)
    return;
  if (!append(state, kind, L"", 0)) {
    sw_free_event(state->event);
    state->event = nullptr;
    sw_fail(state->window->application, ERROR_NOT_ENOUGH_MEMORY);
    return;
  }
  queue_batch(state);
}

void sw_text_end_composition(SWTextState *state) {
  if (!state || state->closed || state->canceling)
    return;
  state->composing = false;
  if (state->editing) {
    if (marked(state->projection) && !append(state, 2, L"", 0))
      state->edit_invalid = true;
    state->projection.marked_start = state->projection.marked_end = -1;
    return;
  }
  if (marked(state->local)) {
    state->local.marked_start = state->local.marked_end = -1;
    standalone(state, 2);
  }
  if (!state->document.present)
    state->local = SWTextDocument{};
}

void sw_text_composition_canceled(SWTextState *state) {
  if (!state || state->closed || state->canceling)
    return;
  state->canceling = true;
  state->composing = false;
  if (state->editing) {
    state->edit_invalid = true;
    sw_text_end_edit(state, true);
  } else if (marked(state->local)) {
    standalone(state, 0);
  }
  state->local.marked_start = state->local.marked_end = -1;
  if (!state->document.present)
    state->local = SWTextDocument{};
  sw_text_store_terminate_composition(state);
  state->canceling = false;
}

void sw_cancel_composition(SWWindow *window) {
  sw_text_composition_canceled(state_of(window));
}

bool sw_text_composing(SWWindow *window) {
  SWTextState *state = state_of(window);
  return state && state->composing;
}

void sw_text_geometry_changed(SWWindow *window) {
  SWTextState *state = state_of(window);
  if (state && state->caret_revision >= 0)
    invalidate_caret(state);
}

void sw_text_store_grant_pending(SWWindow *window);

static HRESULT ensure_manager(SWApplication *application) {
  if (application->text_manager)
    return S_OK;
  SWTextManager *manager = new (std::nothrow) SWTextManager;
  if (!manager)
    return E_OUTOFMEMORY;
  HRESULT hr = CoInitializeEx(NULL, COINIT_APARTMENTTHREADED);
  if (FAILED(hr)) {
    delete manager;
    return hr;
  }
  manager->com_initialized = true;
  hr = CoCreateInstance(CLSID_TF_ThreadMgr, NULL, CLSCTX_INPROC_SERVER,
                        IID_PPV_ARGS(&manager->thread));
  if (SUCCEEDED(hr))
    hr = manager->thread->Activate(&manager->client);
  if (FAILED(hr)) {
    if (manager->thread)
      manager->thread->Release();
    CoUninitialize();
    delete manager;
    return hr;
  }
  application->text_manager = manager;
  return S_OK;
}

void sw_text_application_destroy(SWApplication *application) {
  SWTextManager *manager = manager_of(application);
  application->text_manager = nullptr;
  if (!manager)
    return;
  if (manager->thread) {
    manager->thread->Deactivate();
    manager->thread->Release();
  }
  if (manager->com_initialized)
    CoUninitialize();
  delete manager;
}

void sw_text_focus(SWWindow *window, bool focused) {
  SWTextState *state = state_of(window);
  if (!state || state->closed)
    return;
  state->focused = focused;
  SWTextManager *manager = manager_of(window->application);
  if (!manager)
    return;
  HRESULT hr = S_OK;
  if (focused)
    hr = manager->thread->SetFocus(state->document_manager);
  else {
    sw_cancel_composition(window);
    hr = manager->thread->SetFocus(NULL);
  }
  if (FAILED(hr))
    sw_fail(window->application, (DWORD)hr);
}

void sw_text_window_destroy(SWWindow *window) {
  SWTextState *state = state_of(window);
  window->text = nullptr;
  if (!state)
    return;
  state->closed = true;
  state->composing = false;
  close_text_store(state);
  if (state->event)
    sw_free_event(state->event);
  delete state;
}

static int32_t enable_text(SWWindow *window) {
  if (window->text_connection == INT64_MAX)
    return 1;
  SWTextManager *manager = manager_of(window->application);
  if (!manager && FAILED(ensure_manager(window->application)))
    return 1;
  manager = manager_of(window->application);
  SWTextState *state = new (std::nothrow) SWTextState(window);
  if (!state)
    return 1;
  state->connection = ++window->text_connection;
  state->focused = GetFocus() == window->hwnd;
  state->caret_x = window->caret_x;
  state->caret_y = window->caret_y;
  state->caret_width = window->caret_width;
  state->caret_height = window->caret_height;
  state->caret_known = window->caret_set;
  HRESULT hr = sw_text_store_create(state, manager->client);
  if (FAILED(hr)) {
    delete state;
    return 1;
  }
  window->text = state;
  window->ime_allowed = true;
  SWEvent *event = sw_event(window, 14);
  if (!event) {
    window->ime_allowed = false;
    window->text = nullptr;
    state->closed = true;
    close_text_store(state);
    delete state;
    return 1;
  }
  event->data.flags = 1;
  event->data.a = state->connection;
  sw_enqueue(window->application, event);
  if (state->focused) {
    hr = manager->thread->SetFocus(state->document_manager);
    if (FAILED(hr)) {
      int64_t connection = state->connection;
      window->ime_allowed = false;
      window->text = nullptr;
      state->closed = true;
      close_text_store(state);
      delete state;
      SWEvent *disabled = sw_event(window, 14);
      if (disabled) {
        disabled->data.a = connection;
        sw_enqueue(window->application, disabled);
      }
      return 1;
    }
  }
  return 0;
}

int32_t scoop_window_ime_allowed(SWWindow *window, int32_t allowed) {
  bool enable = allowed != 0;
  if (window->ime_allowed == enable)
    return 0;
  if (enable)
    return enable_text(window);
  SWTextState *state = state_of(window);
  sw_cancel_composition(window);
  int64_t connection = state ? state->connection : window->text_connection;
  window->ime_allowed = false;
  window->caret_set = false;
  window->text = nullptr;
  if (state) {
    state->closed = true;
    close_text_store(state);
    if (state->event)
      sw_free_event(state->event);
    delete state;
  }
  SWEvent *event = sw_event(window, 14);
  if (event) {
    event->data.flags = 0;
    event->data.a = connection;
    sw_enqueue(window->application, event);
  }
  return 0;
}

static int64_t utf16_offset(const uint8_t *text, int64_t length, int64_t offset,
                            LONG *result) {
  if (offset < 0 || offset > length || offset > INT_MAX)
    return 3;
  int count = offset ? MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                                           (const char *)text, (int)offset,
                                           NULL, 0)
                     : 0;
  if (offset && !count)
    return 3;
  *result = count;
  return 0;
}

int64_t scoop_window_publish_text(void *pointer, SWTextSnapshotData data) {
  SWWindow *window = static_cast<SWWindow *>(pointer);
  SWTextState *state = state_of(window);
  if (!window->ime_allowed || !state || state->connection != data.connection)
    return 1;
  if (data.after < 0 || data.after > state->sequence ||
      (data.after && data.after < state->applied) ||
      (state->document.present && data.revision < state->document.revision))
    return 2;
  try {
    std::wstring text;
    WCHAR *wide = sw_wide(data.text, data.length);
    if (!wide)
      return 3;
    text = wide;
    free(wide);
    SWTextDocument next;
    next.text = std::move(text);
    next.present = true;
    next.revision = data.revision;
    if (utf16_offset(data.text, data.length, data.anchor, &next.anchor) ||
        utf16_offset(data.text, data.length, data.cursor, &next.cursor))
      return 3;
    if (data.marked_start < 0) {
      next.marked_start = next.marked_end = -1;
    } else {
      if (utf16_offset(data.text, data.length, data.marked_start,
                       &next.marked_start) ||
          utf16_offset(data.text, data.length, data.marked_end,
                       &next.marked_end) ||
          !marked(next))
        return 3;
    }
    if (state->document.present &&
        state->document.revision == next.revision &&
        (state->document.text != next.text ||
         state->document.anchor != next.anchor ||
         state->document.cursor != next.cursor ||
         state->document.marked_start != next.marked_start ||
         state->document.marked_end != next.marked_end))
      return 2;
    bool changed = !state->document.present ||
                   state->document.revision != next.revision;
    bool notify_text = false;
    bool notify_selection = !state->document.present ||
                            state->document.anchor != next.anchor ||
                            state->document.cursor != next.cursor;
    TS_TEXTCHANGE change = {};
    if (!data.after && changed && state->document.text != next.text) {
      size_t prefix = 0;
      size_t common = std::min(state->document.text.size(), next.text.size());
      while (prefix < common &&
             state->document.text[prefix] == next.text[prefix])
        ++prefix;
      size_t suffix = 0;
      while (suffix < common - prefix &&
             state->document.text[state->document.text.size() - suffix - 1] ==
                 next.text[next.text.size() - suffix - 1])
        ++suffix;
      change.acpStart = (LONG)prefix;
      change.acpOldEnd = (LONG)(state->document.text.size() - suffix);
      change.acpNewEnd = (LONG)(next.text.size() - suffix);
      notify_text = true;
    }
    state->document = std::move(next);
    if (data.after)
      state->applied = data.after;
    if (changed)
      invalidate_caret(state);
    if (state->applied == state->sequence) {
      state->local = state->document;
      if (notify_text)
        sw_text_store_notify_change(state, change);
      if (notify_selection)
        sw_text_store_notify_selection(state);
    }
    return 0;
  } catch (const std::bad_alloc &) {
    return 3;
  } catch (...) {
    return 3;
  }
}

int64_t scoop_window_publish_text_caret(void *pointer, int64_t connection,
                                        int64_t revision, bool available,
                                        double x, double y, double width,
                                        double height) {
  SWWindow *window = static_cast<SWWindow *>(pointer);
  SWTextState *state = state_of(window);
  if (!window->ime_allowed || !state || state->connection != connection ||
      !state->document.present || state->document.revision != revision ||
      state->applied != state->sequence)
    return 1;
  state->caret_x = x;
  state->caret_y = y;
  state->caret_width = width;
  state->caret_height = height;
  state->caret_revision = revision;
  state->caret_known = available;
  sw_text_notify_layout(state);
  return 0;
}

int64_t scoop_window_clear_ime_caret(void *pointer) {
  SWWindow *window = static_cast<SWWindow *>(pointer);
  window->caret_set = false;
  SWTextState *state = state_of(window);
  if (state)
    invalidate_caret(state);
  return 0;
}

int64_t scoop_window_ime_caret(SWWindow *window, double x, double y,
                               double width, double height) {
  if (fabs(x * window->scale) > INT_MAX / 2 ||
      fabs(y * window->scale) > INT_MAX / 2 ||
      fabs((x + width) * window->scale) > INT_MAX / 2 ||
      fabs((y + height) * window->scale) > INT_MAX / 2)
    return ERROR_INVALID_PARAMETER;
  window->caret_x = x;
  window->caret_y = y;
  window->caret_width = width;
  window->caret_height = height;
  window->caret_set = true;
  SWTextState *state = state_of(window);
  if (!state)
    return 0;
  state->caret_x = x;
  state->caret_y = y;
  state->caret_width = width;
  state->caret_height = height;
  state->caret_revision = -1;
  state->caret_known = true;
  sw_text_notify_layout(state);
  return 0;
}

static void commit_character(SWWindow *window, WCHAR character) {
  if (character >= 0xd800 && character <= 0xdbff) {
    window->surrogate = character;
    return;
  }
  WCHAR text[2] = {character, 0};
  int length = 1;
  if (character >= 0xdc00 && character <= 0xdfff) {
    if (!window->surrogate)
      return;
    text[0] = window->surrogate;
    text[1] = character;
    length = 2;
  }
  window->surrogate = 0;
  if (character < 0x20 || character == 0x7f)
    return;
  SWEvent *event = sw_event(window, 13);
  if (!event)
    return;
  int64_t bytes = 0;
  uint8_t *utf8 = sw_utf8(text, length, &bytes);
  if (!utf8) {
    sw_free_event(event);
    sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
    return;
  }
  SWTextEditData edit = {};
  edit.kind = 1;
  edit.anchor = edit.cursor = -1;
  edit.text = utf8;
  edit.text_length = bytes;
  event->edits = static_cast<SWTextEditData *>(malloc(sizeof(edit)));
  if (!event->edits) {
    free(utf8);
    sw_free_event(event);
    sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
    return;
  }
  event->edits[0] = edit;
  event->edit_count = event->edit_capacity = 1;
  sw_enqueue(window->application, event);
}

bool sw_text(SWWindow *window, UINT msg, WPARAM wp, LPARAM lp,
             LRESULT *result) {
  *result = 0;
  switch (msg) {
  case WM_IME_SETCONTEXT:
    *result = DefWindowProcW(window->hwnd, msg, wp, 0);
    return true;
  case WM_IME_STARTCOMPOSITION:
  case WM_IME_ENDCOMPOSITION:
  case WM_IME_COMPOSITION:
  case WM_IME_CHAR:
    return true;
  case WM_CHAR:
    if (!sw_text_composing(window))
      commit_character(window, (WCHAR)wp);
    return true;
  case WM_UNICHAR:
    if (wp == UNICODE_NOCHAR) {
      *result = TRUE;
      return true;
    }
    if (wp > 0x10ffff || (wp >= 0xd800 && wp <= 0xdfff))
      return true;
    if (wp >= 0x10000) {
      commit_character(window, (WCHAR)(0xd800 + ((wp - 0x10000) >> 10)));
      commit_character(window, (WCHAR)(0xdc00 + ((wp - 0x10000) & 1023)));
    } else {
      commit_character(window, (WCHAR)wp);
    }
    return true;
  case WM_SYSCHAR:
  case WM_DEADCHAR:
  case WM_SYSDEADCHAR:
    return true;
  case WM_INPUTLANGCHANGE:
    sw_cancel_composition(window);
    return false;
  case SW_TEXT_LOCK:
    sw_text_store_grant_pending(window);
    return true;
  default:
    return false;
  }
}

int64_t scoop_window_text_count(SWEvent *event) { return event->edit_count; }
SWTextEditData scoop_window_text_edit(SWEvent *event, int64_t index) {
  return event->edits[index];
}
