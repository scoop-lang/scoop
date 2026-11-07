#include "menu.h"

int64_t sw_menu_error(void) {
  DWORD error = GetLastError();
  return error ? error : ERROR_GEN_FAILURE;
}

int32_t scoop_window_menu_capabilities(void) { return 2 | 4; }

SWResult scoop_window_menu_create(SWApplication *app, int64_t identity) {
  SWMenu *menu = calloc(1, sizeof(*menu));
  if (!menu)
    return (SWResult){NULL, ERROR_NOT_ENOUGH_MEMORY};
  menu->application = app;
  menu->identity = identity;
  menu->binding = -1;
  menu->next_command = 1;
  menu->handle = CreatePopupMenu();
  if (!menu->handle) {
    int64_t error = sw_menu_error();
    free(menu);
    return (SWResult){NULL, error};
  }
  MENUINFO info = {.cbSize = sizeof(info),
                   .fMask = MIM_MENUDATA | MIM_STYLE,
                   .dwStyle = MNS_NOTIFYBYPOS,
                   .dwMenuData = (ULONG_PTR)menu};
  if (!SetMenuInfo(menu->handle, &info)) {
    int64_t error = sw_menu_error();
    DestroyMenu(menu->handle);
    free(menu);
    return (SWResult){NULL, error};
  }
  return (SWResult){menu, 0};
}

SWMenuEntry *sw_menu_find(SWMenu *menu, int64_t identity) {
  for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next)
    if (entry->identity == identity)
      return entry;
  return NULL;
}

SWMenuEntry *sw_menu_command(SWMenu *menu, UINT command) {
  for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next)
    if (entry->command == command)
      return entry;
  return NULL;
}

static void free_entry(SWMenuEntry *entry) {
  while (entry->images) {
    SWMenuImage *image = entry->images;
    entry->images = image->next;
    free(image);
  }
  if (entry->bitmap)
    DeleteObject(entry->bitmap);
  free(entry->text);
  free(entry);
}

int64_t scoop_window_menu_clear(SWMenu *menu) {
  while (GetMenuItemCount(menu->handle) > 0)
    if (!DeleteMenu(menu->handle, 0, MF_BYPOSITION))
      return sw_menu_error();
  if (menu->accelerators)
    DestroyAcceleratorTable(menu->accelerators);
  menu->accelerators = NULL;
  free(menu->accelerator_entries);
  menu->accelerator_entries = NULL;
  menu->accelerator_count = 0;
  while (menu->entries) {
    SWMenuEntry *entry = menu->entries;
    menu->entries = entry->next;
    free_entry(entry);
  }
  menu->tail = NULL;
  return 0;
}

void scoop_window_menu_destroy(SWMenu *menu) {
  // Destroying the root also destroys submenus, but never item-owned bitmaps.
  DestroyMenu(menu->handle);
  if (menu->accelerators)
    DestroyAcceleratorTable(menu->accelerators);
  free(menu->accelerator_entries);
  while (menu->entries) {
    SWMenuEntry *entry = menu->entries;
    menu->entries = entry->next;
    free_entry(entry);
  }
  free(menu);
}

int64_t scoop_window_menu_append(SWMenu *menu, int64_t identity, int64_t parent,
                                 int32_t kind, int32_t checked, int32_t enabled,
                                 int32_t role, const uint8_t *text,
                                 int64_t length, int32_t key, int32_t mods) {
  SWMenuEntry *container = parent ? sw_menu_find(menu, parent) : NULL;
  if (parent && (!container || !container->submenu))
    return ERROR_INVALID_PARAMETER;
  if (menu->next_command > UINT_MAX)
    return ERROR_ARITHMETIC_OVERFLOW;
  SWMenuEntry *entry = calloc(1, sizeof(*entry));
  if (!entry)
    return ERROR_NOT_ENOUGH_MEMORY;
  entry->text = sw_wide(text, length);
  if (!entry->text) {
    free(entry);
    return ERROR_NOT_ENOUGH_MEMORY;
  }
  entry->menu = menu;
  entry->identity = identity;
  entry->command = (UINT)menu->next_command++;
  entry->parent = container ? container->submenu : menu->handle;
  entry->kind = kind;
  entry->role = role;
  entry->key = key;
  entry->modifiers = mods;
  entry->checked = checked != 0;
  entry->enabled = enabled != 0;
  entry->live = true;
  int count = GetMenuItemCount(entry->parent);
  if (count < 0) {
    free_entry(entry);
    return sw_menu_error();
  }
  entry->position = (UINT)count;
  if (kind == 3) {
    entry->submenu = CreatePopupMenu();
    if (!entry->submenu) {
      free_entry(entry);
      return sw_menu_error();
    }
    MENUINFO info = {.cbSize = sizeof(info),
                     .fMask = MIM_MENUDATA | MIM_STYLE,
                     .dwStyle = MNS_NOTIFYBYPOS,
                     .dwMenuData = (ULONG_PTR)menu};
    if (!SetMenuInfo(entry->submenu, &info)) {
      int64_t error = sw_menu_error();
      DestroyMenu(entry->submenu);
      free_entry(entry);
      return error;
    }
  }
  MENUITEMINFOW item = {.cbSize = sizeof(item),
                        .fMask = MIIM_ID | MIIM_DATA | MIIM_FTYPE | MIIM_STATE,
                        .fType = kind == 4   ? MFT_SEPARATOR
                                 : kind == 2 ? MFT_RADIOCHECK
                                             : MFT_STRING,
                        .fState = (enabled ? MFS_ENABLED : MFS_DISABLED) |
                                  (checked ? MFS_CHECKED : MFS_UNCHECKED),
                        .wID = entry->command,
                        .dwItemData = (ULONG_PTR)entry};
  if (entry->submenu) {
    item.fMask |= MIIM_SUBMENU;
    item.hSubMenu = entry->submenu;
  }
  if (!InsertMenuItemW(entry->parent, entry->position, TRUE, &item)) {
    int64_t error = sw_menu_error();
    if (entry->submenu)
      DestroyMenu(entry->submenu);
    free_entry(entry);
    return error;
  }
  if (menu->tail)
    menu->tail->next = entry;
  else
    menu->entries = entry;
  menu->tail = entry;
  return kind == 4 ? 0 : sw_menu_label(entry);
}

int64_t scoop_window_menu_image(SWMenu *menu, int64_t identity, int32_t width,
                                int32_t height, const uint8_t *pixels,
                                double logical_width, double logical_height,
                                int32_t template) {
  SWMenuEntry *entry = sw_menu_find(menu, identity);
  if (!entry)
    return ERROR_INVALID_PARAMETER;
  if ((size_t)width > SIZE_MAX / 4 / (size_t)height)
    return ERROR_ARITHMETIC_OVERFLOW;
  size_t bytes = (size_t)width * (size_t)height * 4;
  if (bytes > SIZE_MAX - sizeof(SWMenuImage))
    return ERROR_ARITHMETIC_OVERFLOW;
  SWMenuImage *image = malloc(sizeof(*image) + bytes);
  if (!image)
    return ERROR_NOT_ENOUGH_MEMORY;
  image->next = entry->images;
  image->width = width;
  image->height = height;
  memcpy(image->pixels, pixels, bytes);
  entry->images = image;
  entry->image_width = logical_width;
  entry->image_height = logical_height;
  return 0;
}

int64_t scoop_window_menu_commit(SWMenu *menu) {
  int64_t error = sw_menu_rebuild_keys(menu);
  if (!error)
    error =
        sw_menu_images(menu, menu->window ? GetDpiForWindow(menu->window->hwnd)
                                          : GetDpiForSystem());
  if (!error && menu->binding == 1 && !DrawMenuBar(menu->window->hwnd))
    error = sw_menu_error();
  return error;
}

void scoop_window_menu_live(SWMenu *menu, int64_t identity, int32_t enabled) {
  SWMenuEntry *entry = sw_menu_find(menu, identity);
  if (!entry)
    return;
  if (enabled < 0)
    entry->live = false;
  else
    entry->enabled = enabled != 0;
  EnableMenuItem(entry->parent, entry->position,
                 MF_BYPOSITION |
                     (sw_menu_enabled(entry) ? MF_ENABLED : MF_GRAYED));
}

void *scoop_window_menu_handle(SWMenu *menu) { return menu->handle; }
void *scoop_window_menu_actions(SWMenu *menu) { return NULL; }
int64_t scoop_window_menu_item_for_command(SWMenu *menu, int32_t command) {
  SWMenuEntry *entry = sw_menu_command(menu, (UINT)command);
  return entry && entry->live ? entry->identity : 0;
}
