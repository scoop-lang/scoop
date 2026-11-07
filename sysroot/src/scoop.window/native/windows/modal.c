#include "window.h"

typedef struct {
  SWWindow *window;
  bool ended;
} SWModal;

int32_t scoop_window_management_capabilities(void) {
  return 1 | 2 | 4 | 8 | 16 | 32 | 64 | 256;
}
int64_t scoop_window_raise(SWWindow *window) {
  return SetWindowPos(window->hwnd, HWND_TOP, 0, 0, 0, 0,
                      SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
             ? 0
             : GetLastError();
}
int32_t scoop_window_observed_level(SWWindow *window) {
  return (GetWindowLongPtrW(window->hwnd, GWL_EXSTYLE) & WS_EX_TOPMOST) ? 1
         : window->bottom                                               ? -1
                                                                        : 0;
}
bool sw_modal_input_message(UINT message) {
  return (message >= WM_KEYFIRST && message <= WM_KEYLAST) ||
         (message >= WM_MOUSEFIRST && message <= WM_MOUSELAST) ||
         (message >= WM_NCMOUSEMOVE && message <= WM_NCXBUTTONDBLCLK) ||
         message == WM_SYSCOMMAND || message == WM_IME_CHAR ||
         message == WM_IME_STARTCOMPOSITION || message == WM_IME_COMPOSITION;
}
void scoop_window_input_blocked(SWWindow *window, int32_t blocked) {
  if (window->input_blocked == (blocked != 0))
    return;
  window->input_blocked = blocked != 0;
  if (blocked) {
    window->saved_enabled = IsWindowEnabled(window->hwnd);
    sw_cancel_pointer(window, 3);
    sw_reset_input(window);
    sw_text_focus(window, false);
    EnableWindow(window->hwnd, FALSE);
  } else
    EnableWindow(window->hwnd, window->saved_enabled);
  sw_emit(window, 19);
}
SWResult scoop_window_modal_create(SWWindow *window, int32_t scope,
                                   int32_t presentation) {
  if (presentation)
    return (SWResult){NULL, -1};
  SWModal *session = calloc(1, sizeof(*session));
  if (!session)
    return (SWResult){NULL, ERROR_NOT_ENOUGH_MEMORY};
  session->window = window;
  return (SWResult){session, 0};
}
int64_t scoop_window_modal_start(SWModal *session) {
  if (session->ended)
    return 0;
  ShowWindow(session->window->hwnd, SW_SHOW);
  SetForegroundWindow(session->window->hwnd);
  SetFocus(session->window->hwnd);
  return 0;
}
void scoop_window_modal_end(SWModal *session) {
  if (session->ended)
    return;
  ShowWindow(session->window->hwnd, SW_HIDE);
  session->ended = true;
  sw_wake(session->window->application);
}
int32_t scoop_window_modal_finished(SWModal *session) { return session->ended; }
void scoop_window_modal_release(SWModal *session) { free(session); }
