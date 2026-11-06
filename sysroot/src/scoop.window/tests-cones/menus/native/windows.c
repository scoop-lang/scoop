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

int32_t scoop_test_win_window_menu(HWND window) {
  HMENU menu = GetMenu(window);
  return menu && GetMenuItemCount(menu) == 1;
}

int32_t scoop_test_win_menu_command(HMENU menu) {
  HMENU child = GetSubMenu(menu, 0);
  if (!child || GetMenuItemCount(child) != 2)
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
    if (error || second == first ||
        scoop_window_menu_item_for_command(menu, (int32_t)first) != 0 ||
        scoop_window_menu_item_for_command(menu, (int32_t)second) !=
            identity + 1)
      failure = __LINE__;
  }
  scoop_window_menu_destroy(menu);
  return failure;
}
