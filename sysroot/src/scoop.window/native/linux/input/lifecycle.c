#include "../window.h"

static void focus_enter(GtkEventControllerFocus *controller, gpointer data) {
  sw_ime_update(data);
}
static void focus_leave(GtkEventControllerFocus *controller, gpointer data) {
  SWWindow *window = data;
  sw_reset_input(window);
  sw_ime_update(window);
}

void sw_input_init(SWWindow *window) {
  window->input.cursor_visible = true;
  sw_keyboard_install(window);
  sw_pointer_install(window);
  sw_contact_install(window);
  GtkEventController *focus = gtk_event_controller_focus_new();
  g_signal_connect(focus, "enter", G_CALLBACK(focus_enter), window);
  g_signal_connect(focus, "leave", G_CALLBACK(focus_leave), window);
  gtk_widget_add_controller(window->view, focus);
}

void sw_input_destroy(SWWindow *window) {
  GListModel *controllers = gtk_widget_observe_controllers(window->view);
  for (guint i = 0; i < g_list_model_get_n_items(controllers); ++i) {
    GObject *controller = G_OBJECT(g_list_model_get_item(controllers, i));
    g_signal_handlers_disconnect_by_data(controller, window);
    g_object_unref(controller);
  }
  g_object_unref(controllers);
  g_clear_object(&window->input.click_controller);
  sw_contacts_clear(&window->input.contacts);
  for (size_t index = 0; index < G_N_ELEMENTS(window->input.pressed); ++index)
    free((void *)window->input.pressed[index].text);
}

void sw_reset_input(SWWindow *window) {
  sw_cancel_contacts(window, SWPointerFocusLost);
  sw_cancel_pointer(window, SWPointerFocusLost);
  sw_cancel_composition(window);
  uint64_t locks = window->input.modifiers & (SW_CAPS | SW_NUM | SW_SCROLL |
                                              (1u << 16) | SW_ALT_GRAPH_KNOWN);
  for (size_t code = 0; code < sizeof(window->input.key_down) /
                                   sizeof(window->input.key_down[0]);
       ++code) {
    if (!window->input.key_down[code])
      continue;
    window->input.key_down[code] = false;
    SWEvent *event = sw_event(window, SWKeyboard);
    if (!event)
      continue;
    event->keyboard = window->input.pressed[code];
    window->input.pressed[code].text = NULL;
    event->keyboard.pressed = 0;
    event->keyboard.repeat = 0;
    event->keyboard.synthetic = 1;
    event->keyboard.modifiers = locks;
    sw_enqueue(window->application, event);
  }
  if (window->input.modifiers != locks) {
    window->input.modifiers = locks;
    SWEvent *event = sw_event(window, SWModifiers);
    if (event) {
      event->modifiers = locks;
      sw_enqueue(window->application, event);
    }
  }
}

void sw_focus(SWWindow *window, bool focused) {
  if (window->snapshot.focused == focused)
    return;
  if (!focused) {
    sw_reset_input(window);
    window->application->input.physical_keys_known = false;
    window->application->input.alt_graph = false;
  }
  window->snapshot.focused = focused;
  sw_ime_update(window);
  if (window->configured)
    sw_emit(window, 4);
}
