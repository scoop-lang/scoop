#include "../window.h"

static void enqueue_contact(SWWindow *window, SWContactData data) {
  SWEvent *event = sw_event(window, SWContact);
  if (event) {
    event->contact = data;
    sw_enqueue(window->application, event);
  }
}

static void cancel_contact(SWWindow *window, SWContactEntry *entry,
                           int reason) {
  SWContactData data = entry->data;
  sw_contact_remove(&window->input.contacts, entry);
  data.phase = SWContactCancelled;
  data.cancel_reason = reason;
  enqueue_contact(window, data);
}

void sw_cancel_contacts(SWWindow *window, int reason) {
  while (window->input.contacts)
    cancel_contact(window, window->input.contacts, reason);
}

bool sw_compat_mouse_message(UINT message) {
  if (message == WM_MOUSEWHEEL || message == WM_MOUSEHWHEEL)
    return false;
  if (message != WM_MOUSEMOVE &&
      !(message >= WM_LBUTTONDOWN && message <= WM_XBUTTONDBLCLK))
    return false;
  INPUT_MESSAGE_SOURCE source;
  if (GetCurrentInputMessageSource(&source) &&
      (source.deviceType == IMDT_TOUCH || source.deviceType == IMDT_PEN))
    return true;
  // The documented WISP signature identifies promoted pen/touch mouse input.
  return ((ULONG_PTR)GetMessageExtraInfo() & 0xffffff00u) == 0xff515700u;
}

void sw_contact_message(SWWindow *window, UINT message, WPARAM wp) {
  switch (message) {
  case WM_ENTERMENULOOP:
  case WM_ENTERSIZEMOVE:
    sw_cancel_contacts(window, SWPointerSystemOperation);
    return;
  case WM_POINTERDOWN:
  case WM_POINTERUPDATE:
  case WM_POINTERUP:
  case WM_POINTERLEAVE:
  case WM_POINTERCAPTURECHANGED:
    break;
  default:
    return;
  }
  UINT32 id = GET_POINTERID_WPARAM(wp);
  SWContactEntry *entry = sw_contact_find(window->input.contacts, id);
  if (message == WM_POINTERCAPTURECHANGED || message == WM_POINTERLEAVE) {
    if (entry)
      cancel_contact(window, entry, SWPointerCaptureLost);
    return;
  }
  POINTER_INFO info;
  if (!GetPointerInfo(id, &info)) {
    if (entry)
      cancel_contact(window, entry, SWPointerCaptureLost);
    return;
  }
  if (info.pointerType != PT_TOUCH && info.pointerType != PT_PEN)
    return;
  if (info.pointerFlags & POINTER_FLAG_CANCELED) {
    if (entry)
      cancel_contact(window, entry, SWPointerCaptureLost);
    return;
  }
  if (message == WM_POINTERDOWN && entry) {
    cancel_contact(window, entry, SWPointerCaptureLost);
    entry = NULL;
  }
  if (message != WM_POINTERDOWN && !entry)
    return;
  POINT position = info.ptPixelLocation;
  if (!ScreenToClient(window->hwnd, &position)) {
    if (entry)
      cancel_contact(window, entry, SWPointerCaptureLost);
    return;
  }
  double scale = GetDpiForWindow(window->hwnd) / 96.0;
  if (scale <= 0)
    scale = window->scale;
  SWContactData data = {.id = id,
                        .device = (int64_t)(uintptr_t)info.sourceDevice,
                        .modifiers = (int64_t)sw_modifiers(),
                        .kind = info.pointerType == PT_TOUCH ? SWContactTouch
                                                             : SWContactPen,
                        .phase = message == WM_POINTERDOWN ? SWContactBegan
                                 : message == WM_POINTERUP ? SWContactEnded
                                                           : SWContactMoved,
                        .has_device = info.sourceDevice != NULL,
                        .x = position.x / scale,
                        .y = position.y / scale,
                        .physical_x = position.x,
                        .physical_y = position.y,
                        .scale = scale};
  if (info.pointerType == PT_TOUCH) {
    POINTER_TOUCH_INFO touch;
    if (GetPointerTouchInfo(id, &touch) &&
        (touch.touchMask & TOUCH_MASK_PRESSURE) && touch.pressure <= 1024) {
      data.has_pressure = true;
      data.pressure = touch.pressure / 1024.0;
    }
  } else {
    POINTER_PEN_INFO pen;
    if (GetPointerPenInfo(id, &pen) && (pen.penMask & PEN_MASK_PRESSURE) &&
        pen.pressure <= 1024) {
      data.has_pressure = true;
      data.pressure = pen.pressure / 1024.0;
    }
  }
  if (data.phase == SWContactBegan) {
    entry = sw_contact_add(&window->input.contacts, data);
    if (!entry) {
      sw_fail(window->application, ERROR_NOT_ENOUGH_MEMORY);
      return;
    }
  } else
    entry->data = data;
  if (data.phase == SWContactEnded)
    sw_contact_remove(&window->input.contacts, entry);
  enqueue_contact(window, data);
  // The caller leaves every pointer message to DefWindowProc consistently.
}
