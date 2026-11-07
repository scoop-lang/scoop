#include "window.h"

typedef struct {
  int64_t window;
  int32_t kind, button;
  POINT position;
} SWFrameInput;
typedef struct {
  SWWindow *window;
  SWFrameInput input;
  int32_t kind, edge;
} SWFrameOperation;

SWResult scoop_window_input_copy(SWEvent *event) {
  if (!event->native_input)
    return (SWResult){NULL, 0};
  SWFrameInput *input = malloc(sizeof(*input));
  if (!input)
    return (SWResult){NULL, ERROR_NOT_ENOUGH_MEMORY};
  *input = (SWFrameInput){event->data.window, event->data.kind,
                          (int32_t)event->data.a, event->input_position};
  return (SWResult){input, 0};
}
void scoop_window_input_release(void *input) { free(input); }

SWResult scoop_window_system_operation_prepare(SWWindow *window,
                                               SWFrameInput *input,
                                               int32_t kind, int32_t edge) {
  if (input->window != window->identity ||
      (kind != 2 && (input->kind != 12 || input->button != 0)) ||
      (kind == 1 && (!window->resizable || IsZoomed(window->hwnd) ||
                     IsIconic(window->hwnd))))
    return (SWResult){NULL, -4};
  SWFrameOperation *operation = malloc(sizeof(*operation));
  if (!operation)
    return (SWResult){NULL, ERROR_NOT_ENOUGH_MEMORY};
  *operation = (SWFrameOperation){window, *input, kind, edge};
  window->pressed_buttons = 0;
  if (GetCapture() == window->hwnd)
    ReleaseCapture();
  return (SWResult){operation, 0};
}

int64_t scoop_window_system_operation_run(SWFrameOperation *operation) {
  HWND hwnd = operation->window->hwnd;
  POINT point = operation->input.position;
  if (operation->kind == 2) {
    HMENU menu = GetSystemMenu(hwnd, FALSE);
    if (!menu)
      return -2;
    UINT command = TrackPopupMenuEx(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON,
                                    point.x, point.y, hwnd, NULL);
    if (command)
      SendMessageW(hwnd, WM_SYSCOMMAND, command, 0);
  } else {
    const int edges[] = {WMSZ_LEFT,       WMSZ_RIGHT,      WMSZ_TOP,
                         WMSZ_BOTTOM,     WMSZ_TOPLEFT,    WMSZ_TOPRIGHT,
                         WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT};
    WPARAM command = operation->kind == 0 ? SC_MOVE | HTCAPTION
                                          : SC_SIZE | edges[operation->edge];
    SendMessageW(hwnd, WM_SYSCOMMAND, command, MAKELPARAM(point.x, point.y));
  }
  return 0;
}

void scoop_window_system_operation_release(void *operation) { free(operation); }
