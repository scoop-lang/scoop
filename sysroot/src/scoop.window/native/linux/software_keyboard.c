#include "../software_keyboard.h"
#include "input/delivery.h"

int64_t scoop_window_text_purpose(SWWindow *window, int64_t connection,
                                  int32_t purpose) {
  if (!window->ime_allowed || window->connection != connection)
    return -4;
  GtkInputPurpose values[] = {
      GTK_INPUT_PURPOSE_FREE_FORM, GTK_INPUT_PURPOSE_DIGITS,
      GTK_INPUT_PURPOSE_NUMBER,    GTK_INPUT_PURPOSE_EMAIL,
      GTK_INPUT_PURPOSE_URL,       GTK_INPUT_PURPOSE_PHONE,
      GTK_INPUT_PURPOSE_PASSWORD,  GTK_INPUT_PURPOSE_FREE_FORM};
  if (purpose < 0 || (size_t)purpose >= G_N_ELEMENTS(values))
    return EINVAL;
  GtkInputHints hints =
      purpose == 6 ? GTK_INPUT_HINT_PRIVATE | GTK_INPUT_HINT_NO_SPELLCHECK
                   : GTK_INPUT_HINT_NONE;
  g_object_set(window->ime, "input-purpose", values[purpose], "input-hints",
               hints, NULL);
  return 0;
}
int64_t scoop_window_keyboard_request(SWWindow *window, int64_t connection,
                                      int32_t visible, SWNativeInput *input) {
  if (!window->ime_allowed || window->connection != connection ||
      window->destroying || window->input_blocked ||
      !gtk_widget_has_focus(window->view))
    return 1;
  if (!visible)
    return -1;
  GdkEvent *event =
      input && input->window == window->identity ? input->event : NULL;
  return gtk_im_context_activate_osk(window->ime, event) ? 0 : 1;
}
SWSoftwareKeyboard scoop_window_keyboard_state(SWWindow *window) {
  (void)window;
  return (SWSoftwareKeyboard){0};
}
