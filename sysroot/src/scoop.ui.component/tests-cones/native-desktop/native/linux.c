#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-value"
#include <gtk/gtk.h>
#pragma clang diagnostic pop
#include <stdint.h>
#include <string.h>

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

static int32_t inspect(GtkWidget *bar, GMenuModel *model, int32_t activate) {
  if (!g_menu_model_get_n_items(model))
    return 0;
  GMenuModel *child = g_menu_model_get_item_link(model, 0, G_MENU_LINK_SUBMENU);
  if (!child)
    child = g_menu_model_get_item_link(model, 0, G_MENU_LINK_SECTION);
  if (child) {
    int32_t result = inspect(bar, child, activate);
    g_object_unref(child);
    return result;
  }
  gchar *title = NULL;
  g_menu_model_get_item_attribute(model, 0, G_MENU_ATTRIBUTE_LABEL, "s",
                                  &title);
  int32_t label = !g_strcmp0(title, "App")       ? 1
                  : !g_strcmp0(title, "Window")  ? 2
                  : !g_strcmp0(title, "Changed") ? 3
                                                 : -1;
  g_free(title);
  if (activate) {
    gchar *action = NULL;
    g_menu_model_get_item_attribute(model, 0, G_MENU_ATTRIBUTE_ACTION, "s",
                                    &action);
    GVariant *target = g_menu_model_get_item_attribute_value(
        model, 0, G_MENU_ATTRIBUTE_TARGET, NULL);
    if (action)
      gtk_widget_activate_action_variant(bar, action, target);
    g_free(action);
    if (target)
      g_variant_unref(target);
  }
  return label;
}

int32_t scoop_test_native_ui_menu(void *pointer, int32_t activate) {
  GtkWidget *bar = find_bar(GTK_WIDGET(pointer));
  return bar ? inspect(bar,
                       gtk_popover_menu_bar_get_menu_model(
                           GTK_POPOVER_MENU_BAR(bar)),
                       activate)
             : 0;
}
