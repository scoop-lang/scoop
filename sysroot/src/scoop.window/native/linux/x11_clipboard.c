#include "x11.h"

static size_t chunk_size(SXApplication *x) {
  size_t size = (size_t)xcb_get_maximum_request_length(x->connection) * 4;
  return size > 65536 + 64 ? 65536 : size > 64 ? size - 64 : 1;
}

static void request(SWApplication *app, xcb_selection_request_event_t *event) {
  SXApplication *x = app->backend;
  if (event->selection != x->clipboard || event->owner != x->helper)
    return;
  xcb_selection_notify_event_t reply = {0};
  reply.response_type = XCB_SELECTION_NOTIFY;
  reply.time = event->time;
  reply.requestor = event->requestor;
  reply.selection = event->selection;
  reply.target = event->target;
  xcb_atom_t property = event->property ? event->property : event->target;
  if (x->owns_clipboard && event->target == x->targets) {
    xcb_atom_t targets[] = {x->targets, x->utf8, x->text};
    if (!sx_check(app,
                  xcb_change_property_checked(
                      x->connection, XCB_PROP_MODE_REPLACE, event->requestor,
                      property, XCB_ATOM_ATOM, 32, 3, targets)))
      reply.property = property;
  } else if (x->owns_clipboard &&
             (event->target == x->utf8 || event->target == x->text)) {
    size_t chunk = chunk_size(x);
    if (x->clipboard_length <= chunk) {
      if (!sx_check(app, xcb_change_property_checked(
                             x->connection, XCB_PROP_MODE_REPLACE,
                             event->requestor, property, event->target, 8,
                             x->clipboard_length, x->clipboard_text)))
        reply.property = property;
    } else {
      SXTransfer *transfer = calloc(1, sizeof(*transfer));
      uint8_t *copy = malloc(x->clipboard_length);
      if (transfer && copy) {
        memcpy(copy, x->clipboard_text, x->clipboard_length);
        transfer->window = event->requestor;
        transfer->property = property;
        transfer->target = event->target;
        transfer->text = copy;
        transfer->length = x->clipboard_length;
        uint32_t mask = XCB_EVENT_MASK_PROPERTY_CHANGE;
        uint32_t size = (uint32_t)fmin(UINT32_MAX, transfer->length);
        int error = sx_check(app, xcb_change_window_attributes_checked(
                                      x->connection, event->requestor,
                                      XCB_CW_EVENT_MASK, &mask));
        if (!error)
          error = sx_check(app, xcb_change_property_checked(
                                    x->connection, XCB_PROP_MODE_REPLACE,
                                    event->requestor, property, x->incr, 32, 1,
                                    &size));
        if (!error) {
          transfer->next = x->transfers;
          x->transfers = transfer;
          reply.property = property;
        } else {
          free(transfer);
          free(copy);
        }
      } else {
        free(transfer);
        free(copy);
      }
    }
  }
  sx_check(app, xcb_send_event_checked(x->connection, false, event->requestor,
                                       XCB_EVENT_MASK_NO_EVENT,
                                       (const char *)&reply));
}

static void receive(SWApplication *app) {
  SXApplication *x = app->backend;
  xcb_get_property_reply_t *reply = xcb_get_property_reply(
      x->connection,
      xcb_get_property(x->connection, true, x->helper, x->transfer,
                       XCB_GET_PROPERTY_TYPE_ANY, 0, UINT32_MAX),
      NULL);
  if (!reply) {
    x->incoming.error = EPIPE;
    x->read_done = true;
    return;
  }
  int length = xcb_get_property_value_length(reply);
  const uint8_t *bytes = xcb_get_property_value(reply);
  if (x->read_target == x->targets) {
    x->read_target = 0;
    if (reply->format == 32 && reply->type == XCB_ATOM_ATOM) {
      const xcb_atom_t *targets = (const void *)bytes;
      for (int i = 0; i < length / 4; ++i) {
        if (targets[i] == x->utf8) {
          x->read_target = x->utf8;
          break;
        }
        if (targets[i] == x->text || targets[i] == XCB_ATOM_STRING)
          x->read_target = targets[i];
      }
    }
    if (x->read_target)
      xcb_convert_selection(x->connection, x->helper, x->clipboard,
                            x->read_target, x->transfer, XCB_CURRENT_TIME);
    else
      x->read_done = true;
  } else if (reply->type == x->incr) {
    x->read_incr = true;
    // Deleting the INCR property acknowledges readiness for the first chunk.
  } else if (reply->format == 8 && reply->type != XCB_ATOM_NONE) {
    if ((uint64_t)length >
        (uint64_t)INT64_MAX - (uint64_t)x->incoming.length - 1) {
      x->incoming.error = EOVERFLOW;
      x->read_done = true;
    } else {
      uint8_t *next =
          realloc(x->incoming.bytes, x->incoming.length + length + 1);
      if (!next) {
        x->incoming.error = ENOMEM;
        x->read_done = true;
      } else {
        memcpy(next + x->incoming.length, bytes, length);
        x->incoming.length += length;
        next[x->incoming.length] = 0;
        x->incoming.bytes = next;
        x->read_done = !x->read_incr || !length;
      }
    }
  } else {
    x->incoming.error = EPROTO;
    x->read_done = true;
  }
  free(reply);
}

bool sx_clipboard_event(SWApplication *app, xcb_generic_event_t *event) {
  SXApplication *x = app->backend;
  uint8_t type = event->response_type & 127;
  if (type == XCB_SELECTION_REQUEST) {
    request(app, (void *)event);
    return true;
  }
  if (type == XCB_SELECTION_CLEAR) {
    if (((xcb_selection_clear_event_t *)event)->selection == x->clipboard) {
      x->owns_clipboard = false;
      free(x->clipboard_text);
      x->clipboard_text = NULL;
      x->clipboard_length = 0;
    }
    return true;
  }
  if (type == XCB_SELECTION_NOTIFY) {
    xcb_selection_notify_event_t *selection = (void *)event;
    if (x->reading && selection->requestor == x->helper &&
        selection->selection == x->clipboard) {
      if (selection->property)
        receive(app);
      else
        x->read_done = true;
    }
    return true;
  }
  if (type != XCB_PROPERTY_NOTIFY)
    return false;
  xcb_property_notify_event_t *property = (void *)event;
  if (x->reading && x->read_incr && property->window == x->helper &&
      property->atom == x->transfer &&
      property->state == XCB_PROPERTY_NEW_VALUE) {
    receive(app);
    return true;
  }
  SXTransfer **link = &x->transfers;
  while (*link) {
    SXTransfer *transfer = *link;
    if (property->window != transfer->window ||
        property->atom != transfer->property ||
        property->state != XCB_PROPERTY_DELETE) {
      link = &transfer->next;
      continue;
    }
    size_t length = transfer->length - transfer->offset;
    size_t chunk = chunk_size(x);
    if (length > chunk)
      length = chunk;
    int error = sx_check(
        app, xcb_change_property_checked(x->connection, XCB_PROP_MODE_REPLACE,
                                         transfer->window, transfer->property,
                                         transfer->target, 8, length,
                                         transfer->text + transfer->offset));
    transfer->offset += length;
    if (!length || error) {
      *link = transfer->next;
      free(transfer->text);
      free(transfer);
    }
    return true;
  }
  return false;
}

SWClipboardText sw_x11_clipboard_read(SWApplication *app) {
  SXApplication *x = app->backend;
  xcb_get_selection_owner_reply_t *owner = xcb_get_selection_owner_reply(
      x->connection, xcb_get_selection_owner(x->connection, x->clipboard),
      NULL);
  if (!owner)
    return (SWClipboardText){.error = EPIPE};
  xcb_window_t selection = owner->owner;
  free(owner);
  if (!selection)
    return (SWClipboardText){0};
  if (selection == x->helper && x->owns_clipboard) {
    uint8_t *text = malloc(x->clipboard_length + 1);
    if (!text)
      return (SWClipboardText){.error = ENOMEM};
    memcpy(text, x->clipboard_text, x->clipboard_length);
    text[x->clipboard_length] = 0;
    return (SWClipboardText){text, x->clipboard_length, 0};
  }
  x->reading = true;
  x->read_done = x->read_incr = false;
  x->incoming = (SWClipboardText){0};
  x->read_target = x->targets;
  xcb_convert_selection(x->connection, x->helper, x->clipboard, x->targets,
                        x->transfer, XCB_CURRENT_TIME);
  double deadline = sw_now() + 5;
  while (!x->read_done) {
    int error = sw_x11_flush(app);
    if (!error)
      error = sw_x11_dispatch(app);
    if (error) {
      x->incoming.error = error;
      break;
    }
    if (x->read_done)
      break;
    double remaining = deadline - sw_now();
    if (remaining <= 0) {
      x->incoming.error = ETIMEDOUT;
      break;
    }
    struct pollfd fd = {app->display_fd, POLLIN, 0};
    if (poll(&fd, 1, (int)ceil(remaining * 1000)) < 0 && errno != EINTR) {
      x->incoming.error = errno;
      break;
    }
  }
  x->reading = false;
  SWClipboardText result = x->incoming;
  x->incoming = (SWClipboardText){0};
  if (result.bytes && !result.error && x->read_target == XCB_ATOM_STRING) {
    if (result.length > (INT64_MAX - 1) / 2)
      result.error = EOVERFLOW;
    else {
      uint8_t *utf8 = malloc(result.length * 2 + 1);
      if (!utf8)
        result.error = ENOMEM;
      else {
        int64_t length = 0;
        for (int64_t i = 0; i < result.length; ++i) {
          if (result.bytes[i] >= 128)
            utf8[length++] = 0xc0 | (result.bytes[i] >> 6);
          utf8[length++] = result.bytes[i] < 128
                               ? result.bytes[i]
                               : 0x80 | (result.bytes[i] & 63);
        }
        utf8[length] = 0;
        free(result.bytes);
        result.bytes = utf8;
        result.length = length;
      }
    }
  }
  if (!result.error && result.bytes &&
      !sw_utf8_valid(result.bytes, result.length))
    result.error = EILSEQ;
  return result;
}

int64_t sw_x11_clipboard_write(SWApplication *app, const uint8_t *text,
                               size_t length) {
  SXApplication *x = app->backend;
  uint8_t *copy = malloc(length + 1);
  if (!copy)
    return ENOMEM;
  memcpy(copy, text, length);
  copy[length] = 0;
  int error = sx_check(
      app, xcb_set_selection_owner_checked(x->connection, x->helper,
                                           x->clipboard, XCB_CURRENT_TIME));
  if (!error) {
    xcb_get_selection_owner_reply_t *owner = xcb_get_selection_owner_reply(
        x->connection, xcb_get_selection_owner(x->connection, x->clipboard),
        NULL);
    if (!owner || owner->owner != x->helper)
      error = EIO;
    free(owner);
  }
  if (error) {
    free(copy);
    return error;
  }
  free(x->clipboard_text);
  x->clipboard_text = copy;
  x->clipboard_length = length;
  x->owns_clipboard = true;
  return 0;
}

void sx_clipboard_finish(SWApplication *app) {
  SXApplication *x = app->backend;
  free(x->clipboard_text);
  free(x->incoming.bytes);
  while (x->transfers) {
    SXTransfer *transfer = x->transfers;
    x->transfers = transfer->next;
    free(transfer->text);
    free(transfer);
  }
}
