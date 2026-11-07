#include "../../native/windows/tray.h"

int32_t scoop_test_win_tray_image(void *pointer) {
  STTray *tray = pointer;
  ICONINFO info;
  if (!tray->icon || !GetIconInfo(tray->icon, &info))
    return 0;
  BITMAP bitmap;
  bool valid = GetObjectW(info.hbmColor, sizeof(bitmap), &bitmap) &&
               bitmap.bmBitsPixel == 32 && bitmap.bmWidth > 0;
  DeleteObject(info.hbmColor);
  DeleteObject(info.hbmMask);
  return valid;
}

void scoop_test_win_tray_activate(void *pointer) {
  STTray *tray = pointer;
  SendMessageW(tray->hwnd, WM_APP + 1, MAKELPARAM(32, 48),
               MAKELPARAM(NIN_SELECT, 1));
}

int32_t scoop_test_win_tray_recreate(void *pointer) {
  STTray *tray = pointer;
  NOTIFYICONDATAW data = {.cbSize = sizeof(data), .hWnd = tray->hwnd, .uID = 1};
  if (!Shell_NotifyIconW(NIM_DELETE, &data))
    return 0;
  SendMessageW(tray->hwnd, tray->taskbar_created, 0, 0);
  return tray->available;
}
