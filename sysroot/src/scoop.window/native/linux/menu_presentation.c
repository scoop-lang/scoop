#include "menu.h"

static void unmapped(GtkWidget *widget, gpointer data) {
  SWMenu *menu = data;
  sw_menu_event(menu, 112, 0, 0);
}

void sw_menu_watch(SWMenu *menu, GtkWidget *widget, bool connect) {
  if (GTK_IS_POPOVER(widget)) {
    if (connect &&
        g_object_get_data(G_OBJECT(widget), "scoop-menu-watch") != menu) {
      g_object_set_data(G_OBJECT(widget), "scoop-menu-watch", menu);
      g_signal_connect(widget, "unmap", G_CALLBACK(unmapped), menu);
    } else if (!connect) {
      g_signal_handlers_disconnect_by_data(widget, menu);
      g_object_set_data(G_OBJECT(widget), "scoop-menu-watch", NULL);
    }
  }
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child))
    sw_menu_watch(menu, child, connect);
}

bool sw_menu_tracking(GtkWidget *widget) {
  if (GTK_IS_POPOVER(widget) && gtk_widget_get_mapped(widget))
    return true;
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child))
    if (sw_menu_tracking(child))
      return true;
  return false;
}

int32_t scoop_window_menu_busy(SWMenu *menu) {
  return menu->presenting || menu->external_tracking ||
         (menu->bar && sw_menu_tracking(menu->bar));
}

static void cancel_widget(GtkWidget *widget) {
  for (GtkWidget *child = gtk_widget_get_first_child(widget); child;
       child = gtk_widget_get_next_sibling(child))
    cancel_widget(child);
  if (GTK_IS_POPOVER(widget))
    gtk_popover_popdown(GTK_POPOVER(widget));
}

void sw_menu_dismiss(SWMenu *menu) {
  if (menu->bar)
    cancel_widget(menu->bar);
  if (menu->popup && menu->presenting)
    gtk_popover_popdown(GTK_POPOVER(menu->popup));
}

void scoop_window_menu_cancel(SWMenu *menu) {
  menu->cancelled = true;
  sw_menu_dismiss(menu);
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
    return EINVAL;
  menu->binding = kind;
  menu->cancelled = false;
  menu->serial = serial;
  menu->window = window;
  if (kind == 1) {
    menu->bar = gtk_popover_menu_bar_new_from_model(G_MENU_MODEL(menu->model));
    gtk_widget_insert_action_group(menu->bar, "scoop-menu",
                                   G_ACTION_GROUP(menu->actions));
    adw_toolbar_view_add_top_bar(ADW_TOOLBAR_VIEW(window->toolbar), menu->bar);
    window->menu_bar = menu;
    sw_menu_watch(menu, menu->bar, true);
  }
  return 0;
}

int64_t scoop_window_menu_unbind(SWMenu *menu) {
  if (scoop_window_menu_busy(menu))
    return -2;
  if (menu->bar) {
    sw_menu_watch(menu, menu->bar, false);
    menu->window->menu_bar = NULL;
    adw_toolbar_view_remove(ADW_TOOLBAR_VIEW(menu->window->toolbar), menu->bar);
    menu->bar = NULL;
  }
  if (menu->popup) {
    g_signal_handlers_disconnect_by_data(menu->popup, menu);
    gtk_widget_unparent(menu->popup);
    menu->popup = NULL;
  }
  menu->window = NULL;
  menu->binding = -1;
  return 0;
}

static void popup_closed(GtkPopover *popover, gpointer data) {
  SWMenu *menu = data;
  if (!menu->presenting)
    return;
  menu->presenting = false;
  sw_menu_event(menu, 111, menu->selected, menu->window->identity);
}

int64_t scoop_window_menu_popup(SWMenu *menu, SWWindow *window, double x,
                                double y, double width, double height,
                                int32_t anchor) {
  if (menu->binding != 2 || menu->window != window)
    return EINVAL;
  if (!gtk_widget_get_mapped(window->view))
    return -2;
  if (x < INT_MIN || x > INT_MAX || y < INT_MIN || y > INT_MAX ||
      width > INT_MAX || height > INT_MAX)
    return ERANGE;
  menu->selected = 0;
  menu->popup = gtk_popover_menu_new_from_model(G_MENU_MODEL(menu->model));
  gtk_widget_insert_action_group(menu->popup, "scoop-menu",
                                 G_ACTION_GROUP(menu->actions));
  gtk_widget_set_parent(menu->popup, window->view);
  gtk_popover_set_has_arrow(GTK_POPOVER(menu->popup), FALSE);
  gtk_popover_set_position(GTK_POPOVER(menu->popup), GTK_POS_BOTTOM);
  GdkRectangle rect = {(int)floor(x), (int)floor(y),
                       anchor ? MAX(1, (int)ceil(width)) : 1,
                       anchor ? MAX(1, (int)ceil(height)) : 1};
  gtk_popover_set_pointing_to(GTK_POPOVER(menu->popup), &rect);
  g_signal_connect(menu->popup, "closed", G_CALLBACK(popup_closed), menu);
  menu->presenting = true;
  gtk_popover_popup(GTK_POPOVER(menu->popup));
  return 0;
}
