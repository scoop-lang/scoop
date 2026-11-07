#include "../../../native/windows/menu.h"

// Exercise the C ABI's wide identities directly without manufacturing tens of
// thousands of managed menu items. The application is held by the caller's
// lease.
extern SWResult scoop_window_menu_create(SWApplication *, int64_t);
extern int64_t scoop_window_menu_append(SWMenu *, int64_t, int64_t, int32_t,
                                        int32_t, int32_t, int32_t,
                                        const uint8_t *, int64_t, int32_t,
                                        int32_t);
extern int64_t scoop_window_menu_commit(SWMenu *);
extern int64_t scoop_window_menu_clear(SWMenu *);
extern int64_t scoop_window_menu_item_for_command(SWMenu *, int32_t);
extern void scoop_window_menu_destroy(SWMenu *);

static bool menu_item_matches(HMENU menu, UINT position, UINT type,
                              const WCHAR *expected, bool shortcut) {
  WCHAR text[128] = {0};
  MENUITEMINFOW item = {.cbSize = sizeof(item),
                        .fMask = MIIM_FTYPE | MIIM_STRING,
                        .dwTypeData = text,
                        .cch = sizeof(text) / sizeof(text[0])};
  if (!GetMenuItemInfoW(menu, position, TRUE, &item) || item.fType != type)
    return false;
  size_t length = wcslen(expected);
  return shortcut ? wcsncmp(text, expected, length) == 0 &&
                        text[length] == L'\t' && text[length + 1] != 0
                  : wcscmp(text, expected) == 0;
}

static bool menu_labels_match(HMENU menu) {
  HMENU child = GetSubMenu(menu, 0);
  return menu_item_matches(menu, 0, MFT_STRING, L"File", false) && child &&
         menu_item_matches(child, 0, MFT_STRING, L"Execute", true) &&
         menu_item_matches(child, 1, MFT_STRING, L"Checked", false);
}

int32_t scoop_test_win_window_menu(HWND window) {
  HMENU menu = GetMenu(window);
  return menu && GetMenuItemCount(menu) == 1 && menu_labels_match(menu);
}

static HWND popup_owner;
static UINT_PTR popup_select_timer, popup_cancel_timer;

void scoop_test_win_menu_cancel_input(void) {
  if (popup_select_timer)
    KillTimer(NULL, popup_select_timer);
  if (popup_cancel_timer)
    KillTimer(NULL, popup_cancel_timer);
  popup_select_timer = popup_cancel_timer = 0;
  popup_owner = NULL;
}

static void CALLBACK select_first_item(HWND window, UINT message,
                                       UINT_PTR timer, DWORD time) {
  GUITHREADINFO state = {.cbSize = sizeof(state)};
  if (!GetGUIThreadInfo(GetCurrentThreadId(), &state) ||
      !(state.flags & GUI_INMENUMODE) || state.hwndMenuOwner != popup_owner)
    return;
  KillTimer(NULL, timer);
  popup_select_timer = 0;
  PostMessageW(popup_owner, WM_KEYDOWN, VK_DOWN, 0);
  PostMessageW(popup_owner, WM_KEYDOWN, VK_RIGHT, 0);
  PostMessageW(popup_owner, WM_KEYDOWN, VK_RETURN, 0);
}

static void CALLBACK cancel_popup(HWND window, UINT message, UINT_PTR timer,
                                  DWORD time) {
  scoop_test_win_menu_cancel_input();
  EndMenu();
}

int32_t scoop_test_win_menu_select_input(HWND window) {
  scoop_test_win_menu_cancel_input();
  popup_owner = window;
  popup_select_timer = SetTimer(NULL, 0, 30, select_first_item);
  popup_cancel_timer = SetTimer(NULL, 0, 5000, cancel_popup);
  if (!popup_select_timer || !popup_cancel_timer) {
    scoop_test_win_menu_cancel_input();
    return 0;
  }
  return 1;
}

int32_t scoop_test_win_menu_command(HMENU menu) {
  HMENU child = GetSubMenu(menu, 0);
  if (!child || GetMenuItemCount(child) != 2 || !menu_labels_match(menu))
    return 0;
  MENUITEMINFOW checked = {.cbSize = sizeof(checked), .fMask = MIIM_STATE};
  if (!GetMenuItemInfoW(child, 1, TRUE, &checked) ||
      !(checked.fState & MFS_CHECKED))
    return 0;
  return (int32_t)GetMenuItemID(child, 0);
}

int32_t scoop_test_win_menu_wide_ids(HMENU handle) {
  MENUINFO info = {.cbSize = sizeof(info), .fMask = MIM_MENUDATA};
  if (!GetMenuInfo(handle, &info) || !info.dwMenuData)
    return __LINE__;
  SWMenu *borrowed = (SWMenu *)info.dwMenuData;
  SWResult result =
      scoop_window_menu_create(borrowed->application, INT64_C(1) << 42);
  if (!result.pointer)
    return __LINE__;
  SWMenu *menu = result.pointer;
  int32_t failure = 0;
  int64_t identity = (INT64_C(1) << 40) + 7;
  int64_t error = scoop_window_menu_append(menu, identity, 0, 0, 0, 1, 0,
                                           (const uint8_t *)"Wide", 4, 0, 0);
  if (!error)
    error = scoop_window_menu_commit(menu);
  UINT first = GetMenuItemID(menu->handle, 0);
  if (error ||
      !menu_item_matches(menu->handle, 0, MFT_STRING, L"Wide", false) ||
      scoop_window_menu_item_for_command(menu, (int32_t)first) != identity)
    failure = __LINE__;
  if (!failure) {
    error = scoop_window_menu_clear(menu);
    if (!error)
      error = scoop_window_menu_append(menu, identity + 1, 0, 0, 0, 1, 0,
                                       (const uint8_t *)"Next", 4, 0, 0);
    if (!error)
      error = scoop_window_menu_commit(menu);
    UINT second = GetMenuItemID(menu->handle, 0);
    if (error ||
        !menu_item_matches(menu->handle, 0, MFT_STRING, L"Next", false) ||
        second == first ||
        scoop_window_menu_item_for_command(menu, (int32_t)first) != 0 ||
        scoop_window_menu_item_for_command(menu, (int32_t)second) !=
            identity + 1)
      failure = __LINE__;
  }
  if (!failure) {
    const char *labels[] = {"", "Check & mark", "Radio", "Submenu", ""};
    const WCHAR *expected[] = {L"", L"Check && mark", L"Radio", L"Submenu",
                               L""};
    const UINT types[] = {MFT_STRING, MFT_STRING, MFT_RADIOCHECK, MFT_STRING,
                          MFT_SEPARATOR};
    error = scoop_window_menu_clear(menu);
    for (int kind = 0; !error && kind < 5; ++kind)
      error = scoop_window_menu_append(
          menu, identity + kind + 2, 0, kind, kind == 1 || kind == 2, 1, 0,
          (const uint8_t *)labels[kind], (int64_t)strlen(labels[kind]), 0, 0);
    if (!error)
      error = scoop_window_menu_commit(menu);
    if (error)
      failure = __LINE__;
    for (int kind = 0; !failure && kind < 5; ++kind)
      if (!menu_item_matches(menu->handle, (UINT)kind, types[kind],
                             expected[kind], false))
        failure = __LINE__;
  }
  scoop_window_menu_destroy(menu);
  return failure;
}
