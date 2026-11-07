#include "menu.h"

int32_t scoop_window_menu_capabilities(void) { return 2 | 4; }

static void free_entry(gpointer data) {
  SWMenuEntry *entry = data;
  g_clear_object(&entry->item);
  g_clear_object(&entry->children);
  g_clear_object(&entry->action);
  g_ptr_array_unref(entry->entries);
  g_free(entry);
}

SWResult scoop_window_menu_create(SWApplication *app, int64_t identity) {
  if (app->error)
    return (SWResult){NULL, -3};
  SWMenu *menu = g_new0(SWMenu, 1);
  menu->application = app;
  menu->identity = identity;
  menu->binding = -1;
  menu->model = g_menu_new();
  menu->actions = g_simple_action_group_new();
  menu->entries = g_ptr_array_new_with_free_func(free_entry);
  menu->roots = g_ptr_array_new();
  menu->next = app->menus;
  app->menus = menu;
  return (SWResult){menu, 0};
}

SWMenuEntry *sw_menu_find(SWMenu *menu, int64_t identity) {
  for (guint index = 0; index < menu->entries->len; ++index) {
    SWMenuEntry *entry = g_ptr_array_index(menu->entries, index);
    if (entry->identity == identity)
      return entry;
  }
  return NULL;
}

int64_t scoop_window_menu_clear(SWMenu *menu) {
  g_menu_remove_all(menu->model);
  for (guint index = 0; index < menu->entries->len; ++index) {
    SWMenuEntry *entry = g_ptr_array_index(menu->entries, index);
    if (entry->action)
      g_action_map_remove_action(G_ACTION_MAP(menu->actions),
                                 g_action_get_name(G_ACTION(entry->action)));
  }
  g_ptr_array_set_size(menu->roots, 0);
  g_ptr_array_set_size(menu->entries, 0);
  return 0;
}

void scoop_window_menu_destroy(SWMenu *menu) {
  SWMenu **link = &menu->application->menus;
  while (*link != menu)
    link = &(*link)->next;
  *link = menu->next;
  scoop_window_menu_clear(menu);
  g_ptr_array_unref(menu->roots);
  g_ptr_array_unref(menu->entries);
  g_object_unref(menu->model);
  g_object_unref(menu->actions);
  g_free(menu);
}

static GdkModifierType modifiers(int32_t value) {
  return ((value & 1) ? GDK_CONTROL_MASK : 0) |
         ((value & 2) ? GDK_ALT_MASK : 0) | ((value & 4) ? GDK_SHIFT_MASK : 0) |
         ((value & 8) ? GDK_SUPER_MASK : 0);
}

int64_t scoop_window_menu_append(SWMenu *menu, int64_t identity, int64_t parent,
                                 int32_t kind, int32_t checked, int32_t enabled,
                                 int32_t role, const uint8_t *text,
                                 int64_t length, int32_t key, int32_t mods) {
  SWMenuEntry *container = parent ? sw_menu_find(menu, parent) : NULL;
  if (parent && (!container || !container->children))
    return EINVAL;
  SWMenuEntry *entry = g_new0(SWMenuEntry, 1);
  entry->menu = menu;
  entry->identity = identity;
  entry->kind = kind;
  entry->role = role;
  entry->key = (guint)key;
  entry->modifiers = mods;
  entry->enabled = enabled != 0;
  entry->live = true;
  entry->entries = g_ptr_array_new();
  g_ptr_array_add(menu->entries, entry);
  g_ptr_array_add(container ? container->entries : menu->roots, entry);
  if (kind == 4)
    return 0;
  GString *caption = g_string_sized_new((gsize)length);
  for (int64_t index = 0; index < length; ++index) {
    g_string_append_c(caption, (char)text[index]);
    if (text[index] == '_')
      g_string_append_c(caption, '_');
  }
  char *label = g_string_free(caption, FALSE);
  if (kind == 3) {
    entry->children = g_menu_new();
    entry->item = g_menu_item_new_submenu(label, G_MENU_MODEL(entry->children));
  } else
    entry->item = g_menu_item_new(label, NULL);
  {
    char *name = g_strdup_printf("item%" G_GINT64_FORMAT, identity);
    char *detailed = g_strconcat("scoop-menu.", name, NULL);
    if (kind == 1)
      entry->action = g_simple_action_new_stateful(
          name, NULL, g_variant_new_boolean(checked != 0));
    else if (kind == 2)
      entry->action = g_simple_action_new_stateful(
          name, G_VARIANT_TYPE_STRING,
          g_variant_new_string(checked ? "on" : "off"));
    else
      entry->action = g_simple_action_new(name, NULL);
    g_simple_action_set_enabled(entry->action, enabled != 0);
    g_signal_connect(entry->action, "activate", G_CALLBACK(sw_menu_action),
                     entry);
    g_action_map_add_action(G_ACTION_MAP(menu->actions),
                            G_ACTION(entry->action));
    g_menu_item_set_action_and_target_value(
        entry->item, detailed, kind == 2 ? g_variant_new_string("on") : NULL);
    if (key) {
      char *accelerator = gtk_accelerator_name((guint)key, modifiers(mods));
      g_menu_item_set_attribute(entry->item, "accel", "s", accelerator);
      g_free(accelerator);
    }
    g_free(detailed);
    g_free(name);
  }
  g_menu_item_set_attribute(entry->item, "scoop-id", "x", identity);
  g_menu_item_set_attribute(entry->item, "scoop-kind", "i", kind);
  g_menu_item_set_attribute(entry->item, "scoop-role", "i", role);
  char *original = g_strndup((const char *)text, (gsize)length);
  g_menu_item_set_attribute(entry->item, "scoop-label", "s", original);
  g_free(original);
  g_free(label);
  return 0;
}

int64_t scoop_window_menu_image(SWMenu *menu, int64_t identity, int32_t width,
                                int32_t height, const uint8_t *pixels,
                                double logical_width, double logical_height,
                                int32_t template) {
  SWMenuEntry *entry = sw_menu_find(menu, identity);
  if (!entry || !entry->item)
    return EINVAL;
  size_t area = (size_t)width * (size_t)height;
  if (entry->image_pixels >= area)
    return 0;
  GBytes *bytes = g_bytes_new(pixels, area * 4);
  GdkTexture *texture = gdk_memory_texture_new(
      width, height, GDK_MEMORY_R8G8B8A8, bytes, (gsize)width * 4);
  g_menu_item_set_icon(entry->item, G_ICON(texture));
  entry->image_pixels = area;
  g_object_unref(texture);
  g_bytes_unref(bytes);
  return 0;
}

static void append_level(GMenu *model, GPtrArray *entries) {
  bool sections = false;
  for (guint index = 0; index < entries->len; ++index)
    sections |= ((SWMenuEntry *)g_ptr_array_index(entries, index))->kind == 4;
  GMenu *section = sections ? g_menu_new() : model;
  for (guint index = 0; index < entries->len; ++index) {
    SWMenuEntry *entry = g_ptr_array_index(entries, index);
    if (entry->kind == 4) {
      if (g_menu_model_get_n_items(G_MENU_MODEL(section)))
        g_menu_append_section(model, NULL, G_MENU_MODEL(section));
      g_object_unref(section);
      section = g_menu_new();
    } else {
      if (entry->children)
        append_level(entry->children, entry->entries);
      g_menu_append_item(section, entry->item);
    }
  }
  if (sections) {
    if (g_menu_model_get_n_items(G_MENU_MODEL(section)))
      g_menu_append_section(model, NULL, G_MENU_MODEL(section));
    g_object_unref(section);
  }
}

int64_t scoop_window_menu_commit(SWMenu *menu) {
  append_level(menu->model, menu->roots);
  sw_menu_refresh_commands(menu);
  if (menu->bar)
    sw_menu_watch(menu, menu->bar, true);
  return 0;
}

void scoop_window_menu_live(SWMenu *menu, int64_t identity, int32_t enabled) {
  SWMenuEntry *entry = sw_menu_find(menu, identity);
  if (!entry)
    return;
  if (enabled < 0)
    entry->live = false;
  else
    entry->enabled = enabled != 0;
  if (entry->action)
    g_simple_action_set_enabled(entry->action, entry->live && entry->enabled);
  if (entry->role >= 4)
    sw_menu_refresh_commands(menu);
}

void *scoop_window_menu_handle(SWMenu *menu) { return menu->model; }
void *scoop_window_menu_actions(SWMenu *menu) { return menu->actions; }
int64_t scoop_window_menu_item_for_command(SWMenu *menu, int32_t command) {
  return 0;
}

SWMenuKey scoop_window_menu_resolve_key(int32_t kind, int32_t key,
                                        int32_t mods) {
  if (mods & 16)
    return (SWMenuKey){0, 0, -1};
  guint symbol = 0;
  if (kind == 0)
    symbol = gdk_unicode_to_keyval((guint32)key);
  else if (kind == 1) {
    static const guint named[] = {0,
                                  GDK_KEY_Return,
                                  GDK_KEY_Tab,
                                  GDK_KEY_space,
                                  GDK_KEY_BackSpace,
                                  GDK_KEY_Escape,
                                  GDK_KEY_Delete,
                                  GDK_KEY_Left,
                                  GDK_KEY_Right,
                                  GDK_KEY_Up,
                                  GDK_KEY_Down,
                                  GDK_KEY_Home,
                                  GDK_KEY_End,
                                  GDK_KEY_Page_Up,
                                  GDK_KEY_Page_Down};
    if (key > 0 && key < (int32_t)G_N_ELEMENTS(named))
      symbol = named[key];
    else if (key >= 21 && key <= 40)
      symbol = GDK_KEY_F1 + (guint)(key - 21);
  }
  if (!symbol)
    return (SWMenuKey){0, 0, -1};
  return (SWMenuKey){(int32_t)gdk_keyval_to_lower(symbol), mods, 0};
}
