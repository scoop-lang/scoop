#include "menu.h"
#include <richedit.h>

SWWindow *sw_menu_target(SWMenu *menu) {
  if (menu->window)
    return menu->window;
  HWND hwnd =
      menu->external_tracking ? menu->external_window : GetActiveWindow();
  WCHAR name[64];
  if (!hwnd || !GetClassNameW(hwnd, name, 64) || wcscmp(name, sw_window_class))
    return NULL;
  SWWindow *window = (SWWindow *)GetWindowLongPtrW(hwnd, GWLP_USERDATA);
  return window && window->application == menu->application &&
                 (!menu->external_tracking ||
                  window->identity == menu->external_window_identity)
             ? window
             : NULL;
}

void sw_menu_event(SWMenu *menu, int kind, int64_t item, int64_t window) {
  SWEvent *event = calloc(1, sizeof(*event));
  if (!event) {
    sw_fail(menu->application, ERROR_NOT_ENOUGH_MEMORY);
    return;
  }
  event->data.kind = kind;
  event->data.a = menu->identity;
  event->data.b = item;
  event->data.c = menu->serial;
  event->data.d = window;
  sw_stamp_input(&event->data);
  sw_enqueue(menu->application, event);
}

static HWND menu_focus(SWMenu *menu) {
  return menu->external_tracking ? menu->external_focus : GetFocus();
}

static HWND native_editor(SWMenu *menu, SWWindow *window) {
  HWND focus = menu_focus(menu);
  if (!focus || focus == window->hwnd || !IsChild(window->hwnd, focus))
    return NULL;
  WCHAR name[64];
  if (!GetClassNameW(focus, name, 64))
    return NULL;
  return !_wcsicmp(name, L"Edit") || !_wcsnicmp(name, L"RichEdit", 8) ? focus
                                                                      : NULL;
}

bool sw_menu_enabled(SWMenuEntry *entry) {
  if (!entry->live || entry->kind == 4 || entry->menu->cancelled)
    return false;
  SWWindow *window = sw_menu_target(entry->menu);
  if (entry->role != 1 && window &&
      (!IsWindowEnabled(window->hwnd) || window->destroying))
    return false;
  if (entry->role >= 4) {
    if (!window)
      return false;
    HWND editor = native_editor(entry->menu, window);
    if (editor) {
      DWORD start = 0, end = 0;
      SendMessageW(editor, EM_GETSEL, (WPARAM)&start, (LPARAM)&end);
      bool editable = !(GetWindowLongPtrW(editor, GWL_STYLE) & ES_READONLY);
      switch (entry->role) {
      case 4:
        return editable && start != end;
      case 5:
        return start != end;
      case 6:
        return editable && IsClipboardFormatAvailable(CF_UNICODETEXT);
      case 7:
        return GetWindowTextLengthW(editor) > 0;
      case 8:
        return editable && SendMessageW(editor, EM_CANUNDO, 0, 0);
      case 9:
        return editable && SendMessageW(editor, EM_CANREDO, 0, 0);
      default:
        return false;
      }
    }
    return entry->enabled && menu_focus(entry->menu) == window->hwnd &&
           window->ime_allowed;
  }
  if (!entry->enabled)
    return false;
  if (entry->role == 2)
    return window && window->minimize_button;
  if (entry->role == 3)
    return window && window->maximize_button && window->resizable;
  return true;
}

bool sw_menu_activate(SWMenuEntry *entry) {
  SWMenu *menu = entry->menu;
  if (menu->binding < 0 || entry->kind >= 3 || !sw_menu_enabled(entry))
    return false;
  SWWindow *window = sw_menu_target(menu);
  menu->selected = entry->identity;
  if (!entry->role) {
    sw_menu_event(menu, 110, entry->identity, window ? window->identity : 0);
  } else if (entry->role == 1) {
    SWEvent *event = calloc(1, sizeof(*event));
    if (!event)
      sw_fail(menu->application, ERROR_NOT_ENOUGH_MEMORY);
    else {
      event->data.kind = 101;
      event->data.flags = 1;
      sw_enqueue(menu->application, event);
    }
  } else if (entry->role == 2) {
    ShowWindow(window->hwnd, SW_MINIMIZE);
  } else if (entry->role == 3) {
    ShowWindow(window->hwnd, IsZoomed(window->hwnd) ? SW_RESTORE : SW_MAXIMIZE);
  } else {
    HWND editor = native_editor(menu, window);
    if (editor) {
      static const UINT commands[] = {WM_CUT,    WM_COPY, WM_PASTE,
                                      EM_SETSEL, WM_UNDO, EM_REDO};
      SendMessageW(editor, commands[entry->role - 4], 0,
                   entry->role == 7 ? (LPARAM)-1 : 0);
    } else {
      SWEvent *event = sw_event(window, 16);
      if (event) {
        event->data.a = entry->role;
        sw_enqueue(menu->application, event);
      }
    }
  }
  return true;
}

int32_t scoop_window_menu_activate(SWMenu *menu, int64_t identity) {
  SWMenuEntry *entry = sw_menu_find(menu, identity);
  return entry && sw_menu_activate(entry);
}

static SWMenu *owner_of(SWWindow *window, HMENU handle) {
  SWMenu *candidates[] = {window->menu_bar, window->popup_menu};
  for (int index = 0; index < 2; ++index) {
    SWMenu *menu = candidates[index];
    if (!menu)
      continue;
    if (menu->handle == handle)
      return menu;
    for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next)
      if (entry->submenu == handle)
        return menu;
  }
  return NULL;
}

bool sw_menu_message(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
                     LRESULT *result) {
  SWMenu *menu = NULL;
  switch (message) {
  case WM_ENTERMENULOOP:
    menu = wp ? window->popup_menu : window->menu_bar;
    if (menu)
      menu->tracking = true;
    return false;
  case WM_EXITMENULOOP:
    menu = wp ? window->popup_menu : window->menu_bar;
    if (menu) {
      menu->tracking = false;
      sw_menu_event(menu, 112, 0, 0);
    }
    return false;
  case WM_INITMENU:
  case WM_INITMENUPOPUP:
    menu = owner_of(window, (HMENU)wp);
    if (menu && menu->window == window)
      for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next)
        EnableMenuItem(entry->parent, entry->position,
                       MF_BYPOSITION |
                           (sw_menu_enabled(entry) ? MF_ENABLED : MF_GRAYED));
    return false;
  case WM_MENUCOMMAND: {
    menu = owner_of(window, (HMENU)lp);
    if (!menu || menu->window != window)
      return false;
    MENUITEMINFOW item = {.cbSize = sizeof(item), .fMask = MIIM_DATA};
    if (GetMenuItemInfoW((HMENU)lp, (UINT)wp, TRUE, &item) && item.dwItemData)
      sw_menu_activate((SWMenuEntry *)item.dwItemData);
    *result = 0;
    return true;
  }
  case WM_COMMAND:
    menu = window->application->translating_menu;
    // Accelerator WORDs exist only during this synchronous translation call.
    // They are never interpreted as public IDs or accepted from later messages.
    if (HIWORD(wp) == 1 && !lp && menu && menu->window == window &&
        LOWORD(wp) && LOWORD(wp) <= menu->accelerator_count) {
      sw_menu_activate(menu->accelerator_entries[LOWORD(wp) - 1]);
      *result = 0;
      return true;
    }
    return false;
  default:
    return false;
  }
}
