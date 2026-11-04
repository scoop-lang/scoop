#include "../../native/windows/window.h"

void scoop_window_test_input(HWND hwnd) {
  BYTE original[256], state[256] = {0};
  GetKeyboardState(original);
  SetKeyboardState(state);
  LPARAM key = (0x1e << 16) | 1;
  SendMessageW(hwnd, WM_KEYDOWN, 'A', key);
  SendMessageW(hwnd, WM_KEYDOWN, 'A', key | (1LL << 30));
  SendMessageW(hwnd, WM_KILLFOCUS, 0, 0);
  SendMessageW(hwnd, WM_KEYUP, 'A', key | (3LL << 30));
  state[VK_SHIFT] = state[VK_RSHIFT] = 0x80;
  SetKeyboardState(state);
  SendMessageW(hwnd, WM_KEYDOWN, VK_SHIFT, (0x36 << 16) | 1);
  state[VK_SHIFT] = state[VK_RSHIFT] = 0;
  SetKeyboardState(state);
  SendMessageW(hwnd, WM_KEYUP, VK_SHIFT, (0x36 << 16) | (3LL << 30) | 1);
  SendMessageW(hwnd, WM_KEYDOWN, VK_NUMLOCK, (0x145 << 16) | 1);
  SendMessageW(hwnd, WM_KEYUP, VK_NUMLOCK, (0x145 << 16) | (3LL << 30) | 1);
  SendMessageW(hwnd, WM_KEYDOWN, VK_PAUSE, (0x45 << 16) | 1);
  SendMessageW(hwnd, WM_KEYUP, VK_PAUSE, (0x45 << 16) | (3LL << 30) | 1);
  SendMessageW(hwnd, WM_KEYDOWN, 255, (0x1ff << 16) | 1);
  SendMessageW(hwnd, WM_KEYUP, 255, (0x1ff << 16) | (3LL << 30) | 1);
  SetKeyboardState(original);
  LPARAM position = MAKELPARAM(24, 36);
  SendMessageW(hwnd, WM_MOUSEMOVE, 0, position);
  SendMessageW(hwnd, WM_LBUTTONDBLCLK, MK_LBUTTON, position);
  SendMessageW(hwnd, WM_LBUTTONUP, 0, position);
  POINT screen = {24, 36};
  ClientToScreen(hwnd, &screen);
  SendMessageW(hwnd, WM_MOUSEWHEEL, MAKEWPARAM(0, 60),
               MAKELPARAM(screen.x, screen.y));
  SendMessageW(hwnd, WM_MOUSEHWHEEL, MAKEWPARAM(0, -120),
               MAKELPARAM(screen.x, screen.y));
  SendMessageW(hwnd, WM_MOUSELEAVE, 0, 0);
}

void scoop_window_test_pointer_cancellation(HWND hwnd) {
  const LPARAM inside = MAKELPARAM(24, 36);
  const LPARAM outside = MAKELPARAM(-17, -19);
  SendMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, inside);
  SendMessageW(hwnd, WM_MOUSEMOVE, MK_LBUTTON, outside);
  SendMessageW(hwnd, WM_MOUSELEAVE, 0, 0);
  SendMessageW(hwnd, WM_KILLFOCUS, 0, 0);
  SendMessageW(hwnd, WM_CANCELMODE, 0, 0);
  SendMessageW(hwnd, WM_LBUTTONUP, 0, outside);

  SendMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, inside);
  SendMessageW(hwnd, WM_RBUTTONDOWN, MK_LBUTTON | MK_RBUTTON, inside);
  SendMessageW(hwnd, WM_CANCELMODE, 0, 0);
  SendMessageW(hwnd, WM_LBUTTONUP, MK_RBUTTON, inside);
  SendMessageW(hwnd, WM_RBUTTONUP, 0, inside);

  SendMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, inside);
  ReleaseCapture();
  SendMessageW(hwnd, WM_LBUTTONUP, 0, inside);

  SendMessageW(hwnd, WM_LBUTTONDOWN, MK_LBUTTON, inside);
  SendMessageW(hwnd, WM_LBUTTONUP, 0, inside);
}
