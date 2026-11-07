#define WIN32_LEAN_AND_MEAN
#include <stdint.h>
#include <wchar.h>
#include <windows.h>

int32_t scoop_test_native_ui_menu(void *pointer, int32_t activate) {
  HWND window = pointer;
  HMENU menu = GetSubMenu(GetMenu(window), 0);
  if (!menu)
    return 0;
  wchar_t title[64] = {0};
  GetMenuStringW(menu, 0, title, 64, MF_BYPOSITION);
  int32_t label = !wcscmp(title, L"App")       ? 1
                  : !wcscmp(title, L"Window")  ? 2
                  : !wcscmp(title, L"Changed") ? 3
                                               : -1;
  if (activate)
    SendMessageW(window, WM_MENUCOMMAND, 0, (LPARAM)menu);
  return label;
}
