#include "window.h"
#include <X11/Xatom.h>
#include <gdk/x11/gdkx.h>

typedef struct {
  SWWindow *window;
  bool ended;
} SWModal;

int32_t scoop_window_management_capabilities(void) {
  return 64 | 256 | (sw_application->kind == SW_X11 ? 2 | 4 | 8 | 16 : 0);
}
int64_t scoop_window_raise(SWWindow *window) {
  if (window->application->kind != SW_X11)
    return -1;
  Display *display = gdk_x11_display_get_xdisplay(window->application->display);
  XRaiseWindow(display, gdk_x11_surface_get_xid(sw_surface(window)));
  XFlush(display);
  return 0;
}
int32_t scoop_window_observed_level(SWWindow *window) {
  if (window->application->kind != SW_X11)
    return 2;
  Display *display = gdk_x11_display_get_xdisplay(window->application->display);
  Atom actual;
  int format;
  unsigned long count, remaining;
  unsigned char *bytes = NULL;
  if (XGetWindowProperty(display, gdk_x11_surface_get_xid(sw_surface(window)),
                         XInternAtom(display, "_NET_WM_STATE", False), 0, 256,
                         False, XA_ATOM, &actual, &format, &count, &remaining,
                         &bytes) != Success)
    return 2;
  int result = 0;
  if (format == 32 && actual == XA_ATOM) {
    Atom above = XInternAtom(display, "_NET_WM_STATE_ABOVE", False);
    Atom below = XInternAtom(display, "_NET_WM_STATE_BELOW", False);
    for (unsigned long index = 0; index < count; ++index) {
      Atom value = ((Atom *)bytes)[index];
      if (value == above) {
        result = 1;
        break;
      }
      if (value == below)
        result = -1;
    }
  }
  if (bytes)
    XFree(bytes);
  return result;
}
void scoop_window_input_blocked(SWWindow *window, int32_t blocked) {
  if (window->input_blocked == (blocked != 0))
    return;
  window->input_blocked = blocked != 0;
  if (blocked) {
    window->saved_sensitive = gtk_widget_get_sensitive(window->window);
    sw_cancel_pointer(window, 3);
    sw_reset_input(window);
    gtk_im_context_focus_out(sw_ime_context(window));
    gtk_widget_set_sensitive(window->window, FALSE);
  } else {
    gtk_widget_set_sensitive(window->window, window->saved_sensitive);
    if (window->snapshot.focused && window->ime_allowed)
      gtk_im_context_focus_in(sw_ime_context(window));
  }
  sw_emit(window, 19);
}
SWResult scoop_window_modal_create(SWWindow *window, int32_t scope,
                                   int32_t presentation) {
  if (presentation)
    return (SWResult){NULL, -1};
  SWModal *session = calloc(1, sizeof(*session));
  if (!session)
    return (SWResult){NULL, ENOMEM};
  session->window = window;
  return (SWResult){session, 0};
}
int64_t scoop_window_modal_start(SWModal *session) {
  if (session->ended)
    return 0;
  session->window->requested_visible = true;
  gtk_window_set_modal(GTK_WINDOW(session->window->window), TRUE);
  gtk_window_present(GTK_WINDOW(session->window->window));
  return 0;
}
void scoop_window_modal_end(SWModal *session) {
  if (session->ended)
    return;
  session->window->requested_visible = false;
  gtk_widget_set_visible(session->window->window, FALSE);
  gtk_window_set_modal(GTK_WINDOW(session->window->window), FALSE);
  session->ended = true;
  sw_wake(session->window->application);
}
int32_t scoop_window_modal_finished(SWModal *session) { return session->ended; }
void scoop_window_modal_release(SWModal *session) { free(session); }
