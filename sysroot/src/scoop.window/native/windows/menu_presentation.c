#include "menu.h"

static HMENU clone_menu(SWMenu *owner, HMENU source, bool bar) {
  HMENU target = bar ? CreateMenu() : CreatePopupMenu();
  if (!target)
    return NULL;
  MENUINFO info = {.cbSize = sizeof(info),
                   .fMask = MIM_STYLE | MIM_MENUDATA,
                   .dwStyle = MNS_NOTIFYBYPOS,
                   .dwMenuData = (ULONG_PTR)owner};
  if (!SetMenuInfo(target, &info)) {
    DestroyMenu(target);
    return NULL;
  }
  int count = GetMenuItemCount(source);
  for (int index = 0; index < count; ++index) {
    MENUITEMINFOW item = {.cbSize = sizeof(item),
                          .fMask = MIIM_STRING | MIIM_FTYPE | MIIM_STATE |
                                   MIIM_ID | MIIM_DATA | MIIM_SUBMENU |
                                   MIIM_BITMAP};
    if (!GetMenuItemInfoW(source, (UINT)index, TRUE, &item)) {
      DestroyMenu(target);
      return NULL;
    }
    WCHAR *text = calloc((size_t)item.cch + 1, sizeof(WCHAR));
    if (!text) {
      DestroyMenu(target);
      SetLastError(ERROR_NOT_ENOUGH_MEMORY);
      return NULL;
    }
    item.dwTypeData = text;
    ++item.cch;
    if (!GetMenuItemInfoW(source, (UINT)index, TRUE, &item)) {
      free(text);
      DestroyMenu(target);
      return NULL;
    }
    if (item.hSubMenu) {
      item.hSubMenu = clone_menu(owner, item.hSubMenu, false);
      if (!item.hSubMenu) {
        free(text);
        DestroyMenu(target);
        return NULL;
      }
    }
    BOOL ok = InsertMenuItemW(target, (UINT)index, TRUE, &item);
    free(text);
    if (!ok) {
      if (item.hSubMenu)
        DestroyMenu(item.hSubMenu);
      DestroyMenu(target);
      return NULL;
    }
  }
  return target;
}

static void record_handles(HMENU handle) {
  int count = GetMenuItemCount(handle);
  for (int index = 0; index < count; ++index) {
    MENUITEMINFOW item = {.cbSize = sizeof(item),
                          .fMask = MIIM_DATA | MIIM_SUBMENU};
    if (GetMenuItemInfoW(handle, (UINT)index, TRUE, &item) && item.dwItemData) {
      SWMenuEntry *entry = (SWMenuEntry *)item.dwItemData;
      entry->parent = handle;
      entry->position = (UINT)index;
      entry->submenu = item.hSubMenu;
      if (item.hSubMenu)
        record_handles(item.hSubMenu);
    }
  }
}

int64_t sw_menu_root_kind(SWMenu *menu, bool bar) {
  if (menu->bar == bar)
    return 0;
  HMENU next = clone_menu(menu, menu->handle, bar);
  if (!next)
    return sw_menu_error();
  DestroyMenu(menu->handle);
  menu->handle = next;
  menu->bar = bar;
  record_handles(next);
  return 0;
}

int32_t scoop_window_menu_busy(SWMenu *menu) {
  return menu->tracking || menu->presenting || menu->external_tracking;
}

void scoop_window_menu_cancel(SWMenu *menu) {
  menu->cancelled = true;
  if (menu->tracking || menu->presenting)
    EndMenu();
}

void scoop_window_menu_external_tracking(SWMenu *menu, int32_t tracking) {
  menu->external_tracking = tracking != 0;
  if (!tracking)
    sw_menu_event(menu, 112, 0, 0);
}

int64_t scoop_window_menu_bind(SWMenu *menu, int32_t kind, int64_t serial,
                               SWWindow *window) {
  if (menu->binding >= 0)
    return -2;
  if (kind < 1 || kind > 3)
    return -1;
  if (kind != 3 && !window)
    return ERROR_INVALID_PARAMETER;
  int64_t error = sw_menu_root_kind(menu, kind == 1);
  if (error)
    return error;
  error = sw_menu_images(menu, window ? GetDpiForWindow(window->hwnd)
                                      : GetDpiForSystem());
  if (error)
    return error;
  if (kind == 1) {
    menu->previous = GetMenu(window->hwnd);
    if (!SetMenu(window->hwnd, menu->handle))
      return sw_menu_error();
    if (!DrawMenuBar(window->hwnd)) {
      error = sw_menu_error();
      SetMenu(window->hwnd, menu->previous);
      return error;
    }
    window->menu_bar = menu;
    sw_text_geometry_changed(window);
    sw_emit(window, 2);
  }
  menu->window = window;
  menu->binding = kind;
  menu->cancelled = false;
  menu->serial = serial;
  return 0;
}

int64_t scoop_window_menu_unbind(SWMenu *menu) {
  if (scoop_window_menu_busy(menu))
    return -2;
  if (menu->binding == 1) {
    SWWindow *window = menu->window;
    if (!SetMenu(window->hwnd, menu->previous))
      return sw_menu_error();
    window->menu_bar = NULL;
    menu->previous = NULL;
    DrawMenuBar(window->hwnd);
    sw_text_geometry_changed(window);
    sw_emit(window, 2);
  }
  if (menu->window && menu->window->popup_menu == menu)
    menu->window->popup_menu = NULL;
  menu->window = NULL;
  menu->binding = -1;
  return 0;
}

int64_t scoop_window_menu_popup(SWMenu *menu, SWWindow *window, double x,
                                double y, double width, double height,
                                int32_t anchor) {
  if (menu->binding != 2 || menu->window != window)
    return ERROR_INVALID_PARAMETER;
  double scale = GetDpiForWindow(window->hwnd) / 96.0;
  double native_x = x * scale, native_y = (y + (anchor ? height : 0)) * scale;
  if (native_x < LONG_MIN || native_x > LONG_MAX || native_y < LONG_MIN ||
      native_y > LONG_MAX || width * scale > LONG_MAX ||
      height * scale > LONG_MAX)
    return ERROR_ARITHMETIC_OVERFLOW;
  POINT point = {(LONG)llround(native_x), (LONG)llround(native_y)};
  if (!ClientToScreen(window->hwnd, &point))
    return sw_menu_error();
  if (anchor && ((double)point.x + ceil(width * scale) > LONG_MAX ||
                 (double)point.y - ceil(height * scale) < LONG_MIN))
    return ERROR_ARITHMETIC_OVERFLOW;
  TPMPARAMS params = {.cbSize = sizeof(params)};
  params.rcExclude = (RECT){point.x, point.y - (LONG)llround(height * scale),
                            point.x + (LONG)llround(width * scale), point.y};
  menu->selected = 0;
  menu->presenting = true;
  window->popup_menu = menu;
  SetLastError(0);
  UINT command = (UINT)TrackPopupMenuEx(
      menu->handle,
      TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_LEFTALIGN |
          TPM_TOPALIGN,
      point.x, point.y, window->hwnd, anchor ? &params : NULL);
  DWORD error = command ? 0 : GetLastError();
  menu->presenting = false;
  menu->tracking = false;
  if (command) {
    SWMenuEntry *entry = sw_menu_command(menu, command);
    if (entry)
      sw_menu_activate(entry);
  }
  if (error)
    return error;
  sw_menu_event(menu, 111, menu->selected, window->identity);
  return 0;
}
