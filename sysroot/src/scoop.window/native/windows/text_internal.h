#ifndef SCOOP_WINDOW_WINDOWS_TEXT_INTERNAL_H
#define SCOOP_WINDOW_WINDOWS_TEXT_INTERNAL_H

#include "window.h"

#include <msctf.h>
#include <new>
#include <string>
#include <textstor.h>

struct SWTextDocument {
  std::wstring text;
  LONG anchor = 0;
  LONG cursor = 0;
  LONG marked_start = -1;
  LONG marked_end = -1;
  int64_t revision = 0;
  bool present = false;
};

struct SWTextStore;

#ifndef TS_SS_MULTILINE
#define TS_SS_MULTILINE 0x80
#endif

struct SWTextManager {
  ITfThreadMgr *thread = nullptr;
  TfClientId client = 0;
  bool com_initialized = false;
};

struct SWTextState {
  explicit SWTextState(SWWindow *owner) : window(owner) {}

  SWWindow *window;
  SWTextStore *store = nullptr;
  ITfDocumentMgr *document_manager = nullptr;
  ITfContext *context = nullptr;
  TfEditCookie edit_cookie = 0;
  SWTextDocument document, local, projection;
  SWEvent *event = nullptr;
  int64_t connection = 0, sequence = 0, applied = 0;
  bool editing = false, edit_invalid = false, uses_range = false;
  bool had_marked = false, composing = false, canceling = false;
  bool closed = false, focused = false;
  double caret_x = 0, caret_y = 0, caret_width = 0, caret_height = 0;
  int64_t caret_revision = -1;
  bool caret_known = false;
};

bool sw_text_boundary(const std::wstring &text, LONG offset);
bool sw_text_range(const std::wstring &text, LONG start, LONG end);
const SWTextDocument *sw_text_query(SWTextState *state);
bool sw_text_begin_edit(SWTextState *state);
HRESULT sw_text_replace(SWTextState *state, LONG start, LONG end,
                        const WCHAR *text, ULONG length, bool explicit_range,
                        TS_TEXTCHANGE *change);
HRESULT sw_text_select(SWTextState *state, const TS_SELECTION_ACP &selection);
void sw_text_end_edit(SWTextState *state, bool invalid);
void sw_text_end_composition(SWTextState *state);
void sw_text_composition_canceled(SWTextState *state);
void sw_text_invalidate_caret(SWTextState *state);
void sw_text_notify_layout(SWTextState *state);

HRESULT sw_text_store_create(SWTextState *state, TfClientId client);
ITextStoreACP *sw_text_store_acp(SWTextState *state);
void sw_text_store_close(SWTextState *state);
void sw_text_store_terminate_composition(SWTextState *state);
void sw_text_store_grant_pending(SWTextState *state);
void sw_text_store_notify_change(SWTextState *state,
                                 const TS_TEXTCHANGE &change);
void sw_text_store_notify_selection(SWTextState *state);

#endif
