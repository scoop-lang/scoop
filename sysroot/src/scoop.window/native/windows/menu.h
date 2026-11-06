#ifndef SCOOP_WINDOW_WINDOWS_MENU_H
#define SCOOP_WINDOW_WINDOWS_MENU_H

#include "window.h"

typedef struct {
  int32_t key, modifiers;
  int64_t error;
} SWMenuKey;

typedef struct SWMenuImage {
  struct SWMenuImage *next;
  int width, height;
  uint8_t pixels[];
} SWMenuImage;

typedef struct SWMenuEntry {
  struct SWMenuEntry *next;
  SWMenu *menu;
  int64_t identity;
  UINT command, position;
  HMENU parent, submenu;
  WCHAR *text;
  int32_t kind, role, key, modifiers;
  bool live, enabled, checked;
  SWMenuImage *images;
  HBITMAP bitmap;
  double image_width, image_height;
} SWMenuEntry;

struct SWMenu {
  SWApplication *application;
  SWWindow *window;
  HMENU handle, previous;
  HACCEL accelerators;
  SWMenuEntry *entries, *tail;
  SWMenuEntry **accelerator_entries;
  int accelerator_count;
  HKL layout;
  int64_t identity, serial, selected;
  uint64_t next_command;
  int32_t binding;
  bool bar, tracking, presenting, external_tracking, cancelled;
};

SWMenuEntry *sw_menu_find(SWMenu *menu, int64_t identity);
SWMenuEntry *sw_menu_command(SWMenu *menu, UINT command);
SWWindow *sw_menu_target(SWMenu *menu);
void sw_menu_event(SWMenu *menu, int kind, int64_t item, int64_t window);
bool sw_menu_activate(SWMenuEntry *entry);
bool sw_menu_enabled(SWMenuEntry *entry);
int64_t sw_menu_rebuild_keys(SWMenu *menu);
int64_t sw_menu_label(SWMenuEntry *entry);
int64_t sw_menu_root_kind(SWMenu *menu, bool bar);
int64_t sw_menu_images(SWMenu *menu, UINT dpi);
int64_t sw_menu_error(void);

#endif
