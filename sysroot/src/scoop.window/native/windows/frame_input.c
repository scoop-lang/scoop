#include "window.h"

static const LRESULT buttons[] = {HTCLOSE, HTMINBUTTON, HTMAXBUTTON};

static int button_role(LRESULT hit) {
  return hit == HTCLOSE       ? 0
         : hit == HTMINBUTTON ? 1
         : hit == HTMAXBUTTON ? 2
                              : -1;
}

static bool button_enabled(SWWindow *window, int role) {
  return role == 0   ? window->close_button
         : role == 1 ? window->minimize_button
                     : window->maximize_button && window->resizable;
}

static void button_state(SWWindow *window, int hover, int pressed) {
  int previous_hover = window->frame_hover,
      previous_pressed = window->frame_pressed;
  window->frame_hover = hover;
  window->frame_pressed = pressed;
  for (int role = 0; role < 3; ++role) {
    if ((previous_hover == role) == (hover == role) &&
        (previous_pressed == role) == (pressed == role))
      continue;
    SWEvent *event = sw_event(window, 18);
    if (event) {
      event->data.a = role;
      event->data.flags = (hover == role ? 1 : 0) | (pressed == role ? 2 : 0);
      sw_enqueue(window->application, event);
    }
  }
}

void sw_frame_clear_buttons(SWWindow *window) {
  bool captured = window->frame_button_down >= 0;
  window->frame_button_down = -1;
  button_state(window, -1, -1);
  if (captured && GetCapture() == window->hwnd)
    ReleaseCapture();
}

static int native_button_at(SWWindow *window, POINT screen) {
  TITLEBARINFOEX info = {.cbSize = sizeof(info)};
  SendMessageW(window->hwnd, WM_GETTITLEBARINFOEX, 0, (LPARAM)&info);
  const int indices[] = {5, 2, 3};
  for (int role = 0; role < 3; ++role) {
    int index = indices[role];
    if (!(info.rgstate[index] & STATE_SYSTEM_INVISIBLE) &&
        PtInRect(&info.rgrect[index], screen))
      return role;
  }
  return -1;
}

static LRESULT hit_test(SWWindow *window, LPARAM point) {
  POINT screen = {GET_X_LPARAM(point), GET_Y_LPARAM(point)};
  LRESULT native = 0;
  if (window->frame_mode != 2 &&
      DwmDefWindowProc(window->hwnd, WM_NCHITTEST, 0, point, &native) &&
      button_role(native) >= 0)
    return button_enabled(window, button_role(native)) ? native : HTCLIENT;
  if (window->frame_mode == 1) {
    // DWM can decline hits after maximized NCCALCSIZE clips the client to the
    // work area. Its visible caption buttons still retain their native roles.
    int role = native_button_at(window, screen);
    if (role >= 0)
      return button_enabled(window, role) ? buttons[role] : HTCLIENT;
  }
  POINT client = screen;
  ScreenToClient(window->hwnd, &client);
  double scale = GetDpiForWindow(window->hwnd) / 96.0;
  int role = sw_frame_hit(window->hit_rects, window->hit_count,
                          client.x / scale, client.y / scale);
  if (role >= 10)
    return button_enabled(window, role - 10) ? buttons[role - 10] : HTCLIENT;
  if (role >= 2) {
    const LRESULT edges[] = {HTLEFT,       HTRIGHT,      HTTOP,
                             HTBOTTOM,     HTTOPLEFT,    HTTOPRIGHT,
                             HTBOTTOMLEFT, HTBOTTOMRIGHT};
    return window->resizable && !IsZoomed(window->hwnd) &&
                   !IsIconic(window->hwnd)
               ? edges[role - 2]
               : HTCLIENT;
  }
  if (role == 1)
    return HTCAPTION;
  // Explicit Client entries also exclude native resize/caption fallback.
  for (size_t i = 0; i < window->hit_count; ++i)
    if (sw_frame_contains(window->hit_rects[i].rect, client.x / scale,
                          client.y / scale))
      return HTCLIENT;
  return DefWindowProcW(window->hwnd, WM_NCHITTEST, 0, point);
}

bool sw_frame_input(SWWindow *window, UINT message, WPARAM wp, LPARAM lp,
                    LRESULT *result) {
  if (message == WM_NCHITTEST) {
    *result = hit_test(window, lp);
    return true;
  }
  if (message == WM_CAPTURECHANGED || message == WM_CANCELMODE ||
      message == WM_KILLFOCUS || (message == WM_NCACTIVATE && !wp)) {
    sw_frame_clear_buttons(window);
  } else if (window->frame_button_down >= 0) {
    if (message == WM_KEYDOWN && wp == VK_ESCAPE) {
      sw_frame_clear_buttons(window);
      *result = 0;
      return true;
    }
    if (message == WM_MOUSEMOVE || message == WM_LBUTTONUP ||
        message == WM_NCMOUSEMOVE || message == WM_NCLBUTTONUP) {
      POINT point = {GET_X_LPARAM(lp), GET_Y_LPARAM(lp)};
      if (message == WM_MOUSEMOVE || message == WM_LBUTTONUP)
        ClientToScreen(window->hwnd, &point);
      int down = window->frame_button_down;
      bool inside =
          button_role(hit_test(window, MAKELPARAM(point.x, point.y))) == down &&
          button_enabled(window, down);
      if (message == WM_LBUTTONUP || message == WM_NCLBUTTONUP) {
        sw_frame_clear_buttons(window);
        if (inside) {
          WPARAM command = down == 0                ? SC_CLOSE
                           : down == 1              ? SC_MINIMIZE
                           : IsZoomed(window->hwnd) ? SC_RESTORE
                                                    : SC_MAXIMIZE;
          SendMessageW(window->hwnd, WM_SYSCOMMAND, command,
                       MAKELPARAM(point.x, point.y));
        }
      } else {
        button_state(window, inside ? down : -1, inside ? down : -1);
      }
      *result = 0;
      return true;
    }
  }
  if (message == WM_NCMOUSEMOVE) {
    button_state(window, button_role(wp), window->frame_pressed);
    TRACKMOUSEEVENT tracking = {sizeof(tracking), TME_LEAVE | TME_NONCLIENT,
                                window->hwnd, 0};
    TrackMouseEvent(&tracking);
  } else if (message == WM_NCMOUSELEAVE) {
    button_state(window, -1, -1);
  } else if (message == WM_NCLBUTTONDOWN && button_role(wp) >= 0) {
    int role = button_role(wp);
    *result = 0;
    if (!button_enabled(window, role))
      return true;
    POINT screen = {GET_X_LPARAM(lp), GET_Y_LPARAM(lp)};
    if (window->frame_mode == 0 && native_button_at(window, screen) == role) {
      button_state(window, role, role);
      *result = DefWindowProcW(window->hwnd, message, wp, lp);
      button_state(window, -1, -1);
    } else {
      // Default tracking uses the standard caption's button rectangles, not
      // our hit regions. Keep pumping normally and issue one system command.
      window->frame_button_down = role;
      SetCapture(window->hwnd);
      if (GetCapture() == window->hwnd)
        button_state(window, role, role);
      else
        sw_frame_clear_buttons(window);
    }
    return true;
  }
  return false;
}
