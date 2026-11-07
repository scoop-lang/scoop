#include "tray.h"

typedef struct {
  STMenu *owner;
  DbusmenuMenuitem *item;
  char *action;
  GVariant *target;
} STItem;

struct STMenu {
  STTray *tray;
  GMenuModel *model;
  GActionGroup *actions;
  DbusmenuServer *server;
  DbusmenuMenuitem *root;
  GHashTable *items;
  GPtrArray *models;
  guint idle;
};

static void item_free(gpointer data) {
  STItem *item = data;
  g_signal_handlers_disconnect_by_data(item->item, item);
  g_object_unref(item->item);
  g_free(item->action);
  g_clear_pointer(&item->target, g_variant_unref);
  g_free(item);
}

static void activate(DbusmenuMenuitem *native, guint timestamp, gpointer data) {
  STItem *item = data;
  GActionGroup *actions = item->owner->actions;
  if (!item->owner->tray->closing &&
      g_action_group_has_action(actions, item->action) &&
      g_action_group_get_action_enabled(actions, item->action))
    g_action_group_activate_action(actions, item->action, item->target);
}

static void clear_children(DbusmenuMenuitem *parent) {
  GList *children = g_list_copy(dbusmenu_menuitem_get_children(parent));
  for (GList *entry = children; entry; entry = entry->next)
    dbusmenu_menuitem_child_delete(parent, entry->data);
  g_list_free(children);
}

static void icon_property(DbusmenuMenuitem *item, GVariant *serialized,
                          STTray *tray) {
  dbusmenu_menuitem_property_remove(item, DBUSMENU_MENUITEM_PROP_ICON_NAME);
  dbusmenu_menuitem_property_remove(item, DBUSMENU_MENUITEM_PROP_ICON_DATA);
  if (!serialized)
    return;
  GIcon *icon = g_icon_deserialize(serialized);
  if (!icon)
    return;
  if (G_IS_THEMED_ICON(icon)) {
    const gchar *const *names = g_themed_icon_get_names(G_THEMED_ICON(icon));
    if (names && names[0])
      dbusmenu_menuitem_property_set(item, DBUSMENU_MENUITEM_PROP_ICON_NAME,
                                     names[0]);
  } else if (G_IS_LOADABLE_ICON(icon)) {
    GError *error = NULL;
    GInputStream *stream =
        g_loadable_icon_load(G_LOADABLE_ICON(icon), 0, NULL, NULL, &error);
    if (stream) {
      GByteArray *bytes = g_byte_array_new();
      guint8 buffer[4096];
      gssize length;
      while ((length = g_input_stream_read(stream, buffer, sizeof(buffer), NULL,
                                           &error)) > 0)
        g_byte_array_append(bytes, buffer, (guint)length);
      if (!error)
        dbusmenu_menuitem_property_set_byte_array(
            item, DBUSMENU_MENUITEM_PROP_ICON_DATA, bytes->data, bytes->len);
      g_byte_array_unref(bytes);
      g_object_unref(stream);
    }
    if (error) {
      st_emit(tray, (STEvent){.kind = 6, .error = error->code});
      g_error_free(error);
    }
  }
  g_object_unref(icon);
}

static void shortcut_property(DbusmenuMenuitem *item, const char *accelerator) {
  dbusmenu_menuitem_property_remove(item, DBUSMENU_MENUITEM_PROP_SHORTCUT);
  if (!accelerator)
    return;
  guint key = 0;
  GdkModifierType modifiers = 0;
  if (!gtk_accelerator_parse(accelerator, &key, &modifiers))
    return;
  const char *name = gdk_keyval_name(gdk_keyval_to_upper(key));
  if (!name)
    return;
  GVariantBuilder shortcut;
  g_variant_builder_init(&shortcut, G_VARIANT_TYPE("aas"));
  g_variant_builder_open(&shortcut, G_VARIANT_TYPE("as"));
  if (modifiers & GDK_CONTROL_MASK)
    g_variant_builder_add(&shortcut, "s", DBUSMENU_MENUITEM_SHORTCUT_CONTROL);
  if (modifiers & GDK_ALT_MASK)
    g_variant_builder_add(&shortcut, "s", DBUSMENU_MENUITEM_SHORTCUT_ALT);
  if (modifiers & GDK_SHIFT_MASK)
    g_variant_builder_add(&shortcut, "s", DBUSMENU_MENUITEM_SHORTCUT_SHIFT);
  if (modifiers & GDK_SUPER_MASK)
    g_variant_builder_add(&shortcut, "s", DBUSMENU_MENUITEM_SHORTCUT_SUPER);
  g_variant_builder_add(&shortcut, "s", name);
  g_variant_builder_close(&shortcut);
  dbusmenu_menuitem_property_set_variant(item, DBUSMENU_MENUITEM_PROP_SHORTCUT,
                                         g_variant_builder_end(&shortcut));
}

static void properties(STItem *item, GMenuModel *model, gint index,
                       bool submenu) {
  char *label = NULL, *accelerator = NULL;
  g_menu_model_get_item_attribute(model, index, G_MENU_ATTRIBUTE_LABEL, "s",
                                  &label);
  g_menu_model_get_item_attribute(model, index, "accel", "s", &accelerator);
  dbusmenu_menuitem_property_set(item->item, DBUSMENU_MENUITEM_PROP_LABEL,
                                 label ? label : "");
  shortcut_property(item->item, accelerator);
  g_free(label);
  g_free(accelerator);
  GVariant *icon = g_menu_model_get_item_attribute_value(
      model, index, G_MENU_ATTRIBUTE_ICON, NULL);
  icon_property(item->item, icon, item->owner->tray);
  g_clear_pointer(&icon, g_variant_unref);
  g_clear_pointer(&item->target, g_variant_unref);
  item->target = g_menu_model_get_item_attribute_value(
      model, index, G_MENU_ATTRIBUTE_TARGET, NULL);
  GActionGroup *actions = item->owner->actions;
  bool exists = g_action_group_has_action(actions, item->action);
  dbusmenu_menuitem_property_set_bool(
      item->item, DBUSMENU_MENUITEM_PROP_ENABLED,
      exists && g_action_group_get_action_enabled(actions, item->action));
  dbusmenu_menuitem_property_set_bool(item->item,
                                      DBUSMENU_MENUITEM_PROP_VISIBLE, TRUE);
  dbusmenu_menuitem_property_remove(item->item,
                                    DBUSMENU_MENUITEM_PROP_TOGGLE_TYPE);
  dbusmenu_menuitem_property_remove(item->item,
                                    DBUSMENU_MENUITEM_PROP_TOGGLE_STATE);
  GVariant *state =
      exists ? g_action_group_get_action_state(actions, item->action) : NULL;
  if (state && g_variant_is_of_type(state, G_VARIANT_TYPE_BOOLEAN)) {
    dbusmenu_menuitem_property_set(item->item,
                                   DBUSMENU_MENUITEM_PROP_TOGGLE_TYPE,
                                   DBUSMENU_MENUITEM_TOGGLE_CHECK);
    dbusmenu_menuitem_property_set_int(item->item,
                                       DBUSMENU_MENUITEM_PROP_TOGGLE_STATE,
                                       g_variant_get_boolean(state));
  } else if (state && item->target) {
    dbusmenu_menuitem_property_set(item->item,
                                   DBUSMENU_MENUITEM_PROP_TOGGLE_TYPE,
                                   DBUSMENU_MENUITEM_TOGGLE_RADIO);
    dbusmenu_menuitem_property_set_int(item->item,
                                       DBUSMENU_MENUITEM_PROP_TOGGLE_STATE,
                                       g_variant_equal(state, item->target));
  }
  g_clear_pointer(&state, g_variant_unref);
  if (submenu)
    dbusmenu_menuitem_property_set(item->item,
                                   DBUSMENU_MENUITEM_PROP_CHILD_DISPLAY,
                                   DBUSMENU_MENUITEM_CHILD_DISPLAY_SUBMENU);
  else
    dbusmenu_menuitem_property_remove(item->item,
                                      DBUSMENU_MENUITEM_PROP_CHILD_DISPLAY);
}

static void changed(STMenu *menu);
static void model_changed(GMenuModel *model, gint position, gint removed,
                          gint added, gpointer data) {
  changed(data);
}
static void action_changed(GActionGroup *group, const gchar *action,
                           gpointer data) {
  changed(data);
}
static void enabled_changed(GActionGroup *group, const gchar *action,
                            gboolean enabled, gpointer data) {
  changed(data);
}
static void state_changed(GActionGroup *group, const gchar *action,
                          GVariant *state, gpointer data) {
  changed(data);
}

static void append_level(STMenu *menu, GMenuModel *model,
                         DbusmenuMenuitem *parent, GHashTable *next) {
  g_ptr_array_add(menu->models, g_object_ref(model));
  g_signal_connect(model, "items-changed", G_CALLBACK(model_changed), menu);
  for (gint index = 0; index < g_menu_model_get_n_items(model); ++index) {
    GMenuModel *section =
        g_menu_model_get_item_link(model, index, G_MENU_LINK_SECTION);
    if (section) {
      if (g_menu_model_get_n_items(section) &&
          dbusmenu_menuitem_get_children(parent)) {
        DbusmenuMenuitem *separator = dbusmenu_menuitem_new();
        dbusmenu_menuitem_property_set(separator, DBUSMENU_MENUITEM_PROP_TYPE,
                                       "separator");
        dbusmenu_menuitem_child_append(parent, separator);
        g_object_unref(separator);
      }
      append_level(menu, section, parent, next);
      g_object_unref(section);
      continue;
    }
    char *detailed = NULL;
    if (!g_menu_model_get_item_attribute(model, index, G_MENU_ATTRIBUTE_ACTION,
                                         "s", &detailed))
      continue;
    if (!g_str_has_prefix(detailed, "scoop-menu.")) {
      g_free(detailed);
      continue;
    }
    const char *action = detailed + strlen("scoop-menu.");
    gpointer old_key = NULL, old_item = NULL;
    STItem *item;
    if (g_hash_table_steal_extended(menu->items, action, &old_key, &old_item)) {
      item = old_item;
      g_free(old_key);
    } else {
      item = g_new0(STItem, 1);
      item->owner = menu;
      item->action = g_strdup(action);
      item->item = dbusmenu_menuitem_new();
      g_signal_connect(item->item, DBUSMENU_MENUITEM_SIGNAL_ITEM_ACTIVATED,
                       G_CALLBACK(activate), item);
    }
    clear_children(item->item);
    GMenuModel *submenu =
        g_menu_model_get_item_link(model, index, G_MENU_LINK_SUBMENU);
    properties(item, model, index, submenu != NULL);
    g_hash_table_insert(next, g_strdup(action), item);
    dbusmenu_menuitem_child_append(parent, item->item);
    if (submenu) {
      append_level(menu, submenu, item->item, next);
      g_object_unref(submenu);
    }
    g_free(detailed);
  }
}

static void unwatch_models(STMenu *menu) {
  for (guint index = 0; index < menu->models->len; ++index)
    g_signal_handlers_disconnect_by_data(g_ptr_array_index(menu->models, index),
                                         menu);
  g_ptr_array_set_size(menu->models, 0);
}

static void rebuild(STMenu *menu) {
  unwatch_models(menu);
  clear_children(menu->root);
  GHashTableIter iterator;
  gpointer value;
  g_hash_table_iter_init(&iterator, menu->items);
  while (g_hash_table_iter_next(&iterator, NULL, &value))
    clear_children(((STItem *)value)->item);
  GHashTable *next =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, item_free);
  append_level(menu, menu->model, menu->root, next);
  g_hash_table_unref(menu->items);
  menu->items = next;
}

static gboolean update(gpointer data) {
  STMenu *menu = data;
  menu->idle = 0;
  rebuild(menu);
  return G_SOURCE_REMOVE;
}

static void changed(STMenu *menu) {
  if (!menu->idle)
    menu->idle = g_idle_add_full(G_PRIORITY_DEFAULT_IDLE, update, menu, NULL);
}

STMenu *st_menu_new(STTray *tray, GMenuModel *model, GActionGroup *actions) {
  STMenu *menu = g_new0(STMenu, 1);
  menu->tray = tray;
  menu->model = g_object_ref(model);
  menu->actions = g_object_ref(actions);
  menu->items =
      g_hash_table_new_full(g_str_hash, g_str_equal, g_free, item_free);
  menu->models = g_ptr_array_new_with_free_func(g_object_unref);
  menu->root = dbusmenu_menuitem_new();
  rebuild(menu);
  menu->server = dbusmenu_server_new(tray->menu_path);
  dbusmenu_server_set_root(menu->server, menu->root);
  g_signal_connect(actions, "action-added", G_CALLBACK(action_changed), menu);
  g_signal_connect(actions, "action-removed", G_CALLBACK(action_changed), menu);
  g_signal_connect(actions, "action-enabled-changed",
                   G_CALLBACK(enabled_changed), menu);
  g_signal_connect(actions, "action-state-changed", G_CALLBACK(state_changed),
                   menu);
  return menu;
}

void st_menu_free(STMenu *menu) {
  if (!menu)
    return;
  if (menu->idle)
    g_source_remove(menu->idle);
  g_signal_handlers_disconnect_by_data(menu->actions, menu);
  unwatch_models(menu);
  g_object_unref(menu->server);
  g_object_unref(menu->root);
  g_hash_table_unref(menu->items);
  g_ptr_array_unref(menu->models);
  g_object_unref(menu->actions);
  g_object_unref(menu->model);
  g_free(menu);
}
