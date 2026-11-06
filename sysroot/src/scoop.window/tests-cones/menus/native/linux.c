#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <adwaita.h>
#pragma clang diagnostic pop
#include <stdint.h>
#include <string.h>

static gboolean find_item(GMenuModel *model, int64_t identity, char **action) {
  int count = g_menu_model_get_n_items(model);
  for (int index = 0; index < count; ++index) {
    gint64 value = 0;
    if (g_menu_model_get_item_attribute(model, index, "scoop-id", "x",
                                        &value) &&
        value == identity)
      return g_menu_model_get_item_attribute(model, index, "action", "s",
                                             action);
    const char *links[] = {G_MENU_LINK_SUBMENU, G_MENU_LINK_SECTION};
    for (int link = 0; link < 2; ++link) {
      GMenuModel *child = g_menu_model_get_item_link(model, index, links[link]);
      if (!child)
        continue;
      gboolean found = find_item(child, identity, action);
      g_object_unref(child);
      if (found)
        return TRUE;
    }
  }
  return FALSE;
}

int32_t scoop_test_gtk_menu_model(GMenuModel *model, GActionGroup *actions,
                                  int64_t checked) {
  char *action = NULL;
  if (!find_item(model, checked, &action))
    return __LINE__;
  const char *name = strchr(action, '.');
  GVariant *state =
      name ? g_action_group_get_action_state(actions, name + 1) : NULL;
  gboolean ok = state && g_variant_is_of_type(state, G_VARIANT_TYPE_BOOLEAN) &&
                g_variant_get_boolean(state);
  g_clear_pointer(&state, g_variant_unref);
  g_free(action);
  return ok ? 0 : __LINE__;
}

int32_t scoop_test_gtk_menu_shortcut(GtkWidget *view) {
  GListModel *controllers = gtk_widget_observe_controllers(view);
  gboolean handled = FALSE;
  for (guint index = 0; index < g_list_model_get_n_items(controllers);
       ++index) {
    GtkEventController *controller = g_list_model_get_item(controllers, index);
    if (GTK_IS_EVENT_CONTROLLER_KEY(controller))
      g_signal_emit_by_name(controller, "key-pressed", GDK_KEY_o, 31,
                            GDK_CONTROL_MASK, &handled);
    g_object_unref(controller);
    if (handled)
      break;
  }
  g_object_unref(controllers);
  return handled;
}

static GtkWidget *find_bar(GtkWidget *widget) {
  if (GTK_IS_POPOVER_MENU_BAR(widget))
    return widget;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child)) {
    GtkWidget *bar = find_bar(child);
    if (bar)
      return bar;
  }
  return NULL;
}

int32_t scoop_test_gtk_menu_open(GtkWidget *view) {
  GtkWidget *bar = find_bar(GTK_WIDGET(gtk_widget_get_root(view)));
  GtkWidget *item = bar ? gtk_widget_get_first_child(bar) : NULL;
  return item && gtk_widget_activate(item);
}

int32_t scoop_test_gtk_popup(GtkWidget *view, int64_t identity) {
  for (GtkWidget *child = gtk_widget_get_first_child(view); child;
       child = gtk_widget_get_next_sibling(child)) {
    if (!GTK_IS_POPOVER(child) || !gtk_widget_get_mapped(child))
      continue;
    if (!identity)
      return 1;
    char *name = g_strdup_printf("scoop-menu.item%" G_GINT64_FORMAT, identity);
    gboolean activated = gtk_widget_activate_action(child, name, NULL);
    g_free(name);
    return activated;
  }
  return 0;
}
