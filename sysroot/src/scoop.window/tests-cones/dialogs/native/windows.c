#define WIN32_LEAN_AND_MEAN
#include <stdint.h>
#include <wchar.h>
#include <windows.h>

typedef struct {
  HWND owner;
  int action, result;
} Query;
static BOOL CALLBACK inspect(HWND window, LPARAM context) {
  Query *query = (Query *)context;
  WCHAR title[128], type[64];
  if (!IsWindowVisible(window) || !GetWindowTextW(window, title, 128) ||
      wcscmp(title, L"Scoop file probe") || !GetClassNameW(window, type, 64) ||
      wcscmp(type, L"#32770") ||
      (query->owner && GetWindow(window, GW_OWNER) != query->owner))
    return TRUE;
  query->result += 1;
  if (query->action == 1) {
    PostMessageW(window, WM_CLOSE, 0, 0);
    query->result = 1;
    return FALSE;
  }
  if (query->action == 2) {
    query->result = !IsWindowEnabled(GetWindow(window, GW_OWNER));
    return FALSE;
  }
  return TRUE;
}
int32_t scoop_test_file_dialog(HWND owner, int32_t action, int32_t kind) {
  if (action == 4)
    return 0;
  if (action == 3)
    return IsWindowEnabled(owner);
  Query query = {owner, action, 0};
  EnumThreadWindows(GetCurrentThreadId(), inspect, (LPARAM)&query);
  return query.result;
}

int32_t scoop_test_file_choose(const uint8_t *path, int64_t length) {
  return 0;
}
