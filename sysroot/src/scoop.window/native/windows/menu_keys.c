#include "menu.h"

static WCHAR folded(WCHAR character) {
  WCHAR result;
  if (LCMapStringEx(LOCALE_NAME_INVARIANT, LCMAP_LOWERCASE, &character, 1,
                    &result, 1, NULL, NULL, 0) != 1)
    return 0;
  return result;
}

static UINT character_key(WCHAR character, int32_t mods, HKL layout) {
  SHORT scan = VkKeyScanExW(character, layout);
  // AltGraph-generated text is not a Ctrl+Alt menu accelerator.
  if (scan == -1 || (HIBYTE(scan) & 6))
    return 0;
  BYTE state[256] = {0};
  state[VK_SHIFT] = (mods & 4) ? 0x80 : 0;
  WCHAR result[8];
  UINT key = LOBYTE(scan);
  int count = ToUnicodeEx(key, MapVirtualKeyExW(key, MAPVK_VK_TO_VSC, layout),
                          state, result, 8, 4, layout);
  return count == 1 && folded(result[0]) == character ? key : 0;
}

SWMenuKey scoop_window_menu_resolve_key(int32_t kind, int32_t key,
                                        int32_t mods) {
  if (mods & (8 | 16))
    return (SWMenuKey){0, 0, -1};
  UINT native = 0;
  WCHAR character = 0;
  if (kind == 0) {
    if (key < 32 || key > 0xffff || (key >= 0xd800 && key <= 0xdfff))
      return (SWMenuKey){0, 0, -1};
    character = folded((WCHAR)key);
    if (character)
      native = character_key(character, mods, GetKeyboardLayout(0));
  } else if (kind == 1) {
    static const UINT named[] = {0,        VK_RETURN, VK_TAB,    VK_SPACE,
                                 VK_BACK,  VK_ESCAPE, VK_DELETE, VK_LEFT,
                                 VK_RIGHT, VK_UP,     VK_DOWN,   VK_HOME,
                                 VK_END,   VK_PRIOR,  VK_NEXT};
    if (key > 0 && key < (int32_t)(sizeof(named) / sizeof(named[0])))
      native = named[key];
    else if (key >= 21 && key <= 40)
      native = VK_F1 + (UINT)(key - 21);
    if (native == VK_SPACE)
      character = L' ';
  }
  if (!native)
    return (SWMenuKey){0, 0, -1};
  return (SWMenuKey){(int32_t)(((uint32_t)character << 16) | native), mods, 0};
}

static UINT resolved_key(SWMenuEntry *entry, HKL layout) {
  WCHAR character = (WCHAR)((uint32_t)entry->key >> 16);
  return character ? character_key(character, entry->modifiers, layout)
                   : (UINT)entry->key;
}

static int key_name(UINT key, WCHAR *text, int capacity) {
  UINT scan = MapVirtualKeyW(key, MAPVK_VK_TO_VSC);
  LONG bits = (LONG)(scan << 16);
  if (key == VK_DELETE || key == VK_INSERT || key == VK_HOME || key == VK_END ||
      key == VK_PRIOR || key == VK_NEXT || (key >= VK_LEFT && key <= VK_DOWN))
    bits |= 1L << 24;
  return GetKeyNameTextW(bits, text, capacity);
}

int64_t sw_menu_label(SWMenuEntry *entry) {
  size_t length = wcslen(entry->text);
  if (length > (SIZE_MAX / sizeof(WCHAR) - 256) / 2)
    return ERROR_ARITHMETIC_OVERFLOW;
  WCHAR *label = calloc(length * 2 + 256, sizeof(WCHAR));
  if (!label)
    return ERROR_NOT_ENOUGH_MEMORY;
  size_t offset = 0;
  for (size_t index = 0; index < length; ++index) {
    label[offset++] = entry->text[index];
    if (entry->text[index] == L'&')
      label[offset++] = L'&';
  }
  UINT key = entry->key ? resolved_key(entry, GetKeyboardLayout(0)) : 0;
  if (key) {
    label[offset++] = L'\t';
    static const UINT modifier_keys[] = {VK_CONTROL, VK_MENU, VK_SHIFT};
    for (int index = 0; index < 3; ++index)
      if (entry->modifiers & (1 << index)) {
        offset += (size_t)key_name(modifier_keys[index], label + offset, 50);
        label[offset++] = L'+';
      }
    WCHAR character = (WCHAR)((uint32_t)entry->key >> 16);
    if (character && character != L' ') {
      WCHAR upper = character;
      LCMapStringEx(LOCALE_NAME_INVARIANT, LCMAP_UPPERCASE, &character, 1,
                    &upper, 1, NULL, NULL, 0);
      label[offset++] = upper;
    } else {
      offset += (size_t)key_name(key, label + offset, 80);
    }
  }
  label[offset] = 0;
  MENUITEMINFOW item = {
      .cbSize = sizeof(item), .fMask = MIIM_STRING, .dwTypeData = label};
  BOOL ok = SetMenuItemInfoW(entry->parent, entry->position, TRUE, &item);
  int64_t error = ok ? 0 : sw_menu_error();
  free(label);
  return error;
}

int64_t sw_menu_rebuild_keys(SWMenu *menu) {
  size_t count = 0;
  for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next)
    count += entry->key != 0;
  if (count > UINT16_MAX)
    return ERROR_TOO_MANY_CMDS;
  ACCEL *values = count ? calloc(count, sizeof(*values)) : NULL;
  SWMenuEntry **entries = count ? calloc(count, sizeof(*entries)) : NULL;
  if (count && (!values || !entries)) {
    free(values);
    free(entries);
    return ERROR_NOT_ENOUGH_MEMORY;
  }
  HKL layout = GetKeyboardLayout(0);
  int index = 0;
  for (SWMenuEntry *entry = menu->entries; entry; entry = entry->next) {
    if (!entry->key)
      continue;
    UINT key = resolved_key(entry, layout);
    int64_t error = sw_menu_label(entry);
    if (error) {
      free(values);
      free(entries);
      return error;
    }
    if (!key)
      continue;
    values[index] =
        (ACCEL){.fVirt = FVIRTKEY | ((entry->modifiers & 1) ? FCONTROL : 0) |
                         ((entry->modifiers & 2) ? FALT : 0) |
                         ((entry->modifiers & 4) ? FSHIFT : 0),
                .key = (WORD)key,
                .cmd = (WORD)(index + 1)};
    entries[index++] = entry;
  }
  HACCEL accelerators = index ? CreateAcceleratorTableW(values, index) : NULL;
  free(values);
  if (index && !accelerators) {
    free(entries);
    return sw_menu_error();
  }
  if (menu->accelerators)
    DestroyAcceleratorTable(menu->accelerators);
  free(menu->accelerator_entries);
  menu->accelerators = accelerators;
  menu->accelerator_entries = entries;
  menu->accelerator_count = index;
  menu->layout = layout;
  return 0;
}

bool sw_menu_translate(SWApplication *app, MSG *message) {
  if (message->message != WM_KEYDOWN && message->message != WM_SYSKEYDOWN)
    return false;
  HWND root = GetAncestor(message->hwnd, GA_ROOT);
  WCHAR name[64];
  if (!GetClassNameW(root, name, 64) || wcscmp(name, sw_window_class))
    return false;
  SWWindow *window = (SWWindow *)GetWindowLongPtrW(root, GWLP_USERDATA);
  if (!window || window->application != app || !window->menu_bar ||
      !IsWindowEnabled(root) || sw_text_composing(window))
    return false;
  // Keep the native Ctrl+Alt text path for AltGraph layouts.
  if ((GetKeyState(VK_RMENU) & 0x8000) && (GetKeyState(VK_CONTROL) & 0x8000))
    return false;
  if ((GetKeyState(VK_LWIN) | GetKeyState(VK_RWIN)) & 0x8000)
    return false;
  SWMenu *menu = window->menu_bar;
  if (menu->layout != GetKeyboardLayout(0)) {
    int64_t error = sw_menu_rebuild_keys(menu);
    if (error) {
      sw_fail(app, (DWORD)error);
      return false;
    }
  }
  if (!menu->accelerators)
    return false;
  SWMenu *previous = app->translating_menu;
  app->translating_menu = menu;
  BOOL result = TranslateAcceleratorW(root, menu->accelerators, message);
  app->translating_menu = previous;
  return result != 0;
}
