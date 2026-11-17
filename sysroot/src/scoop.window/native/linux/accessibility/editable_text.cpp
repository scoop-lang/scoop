#include "provider.h"
#include <algorithm>

namespace swax::atspi {
static bool replace(const ElementPtr &element,
                    const std::shared_ptr<const State> &before, int64_t first,
                    int64_t last, const std::string &text) {
  if (!(before->data.actions & 32) ||
      (before->data.flags & (SWAXReadOnly | SWAXProtected)))
    unsupported("This element does not support public range editing");
  if (!actionResult(
          element->perform(5, {}, first, last, before->data.content_revision)))
    return false;
  auto current = element->state();
  if (!current || current->data.text_revision != before->data.text_revision ||
      current->documentEpoch != before->documentEpoch ||
      std::min(current->data.anchor, current->data.cursor) != first ||
      std::max(current->data.anchor, current->data.cursor) != last)
    unavailable();
  return actionResult(
      element->perform(6, text, first, last, current->data.content_revision));
}
struct Paste {
  std::shared_ptr<WindowHost> host;
  ElementPtr element;
  std::shared_ptr<const State> before;
  int64_t offset;
  GDBusMethodInvocation *invocation;
  ~Paste() { g_object_unref(invocation); }
};
static void pasteReady(GObject *clipboard, GAsyncResult *result,
                       void *context) noexcept {
  std::unique_ptr<Paste> pending(static_cast<Paste *>(context));
  GError *error = nullptr;
  gchar *value =
      gdk_clipboard_read_text_finish(GDK_CLIPBOARD(clipboard), result, &error);
  try {
    auto current = pending->element->state();
    if (!pending->host->window || !current ||
        current->data.text_revision != pending->before->data.text_revision ||
        current->documentEpoch != pending->before->documentEpoch)
      unavailable();
    if (error)
      throw Failure("org.a11y.atspi.Error.Failed", error->message);
    bool accepted = value && replace(pending->element, current, pending->offset,
                                     pending->offset, value);
    g_dbus_method_invocation_return_value(pending->invocation,
                                          g_variant_new("(b)", accepted));
  } catch (const Failure &failure) {
    g_dbus_method_invocation_return_dbus_error(pending->invocation,
                                               failure.name, failure.what());
  } catch (...) {
    g_dbus_method_invocation_return_dbus_error(pending->invocation,
                                               "org.a11y.atspi.Error.Failed",
                                               "Clipboard insertion failed");
  }
  g_clear_error(&error);
  g_free(value);
}
static bool copy(Request &request, const std::string &text) {
  if (!request.element->exposed(true) || !request.host->window)
    return false;
  auto *bytes = g_bytes_new(text.data(), text.size());
  auto *provider =
      gdk_content_provider_new_for_bytes("text/plain;charset=utf-8", bytes);
  g_bytes_unref(bytes);
  auto *clipboard =
      gdk_display_get_clipboard(request.host->window->application->display);
  const bool accepted = gdk_clipboard_set_content(clipboard, provider);
  g_object_unref(provider);
  return accepted;
}
GVariant *editableText(Request &request, const char *method,
                       GVariant *parameters) {
  if (!strcmp(method, "SetTextContents")) {
    const char *text;
    g_variant_get(parameters, "(&s)", &text);
    return g_variant_new("(b)",
                         actionResult(request.element->perform(2, text)));
  }
  const auto &content = publicContent(*request.state);
  if (!strcmp(method, "InsertText")) {
    int index, length;
    const char *text;
    g_variant_get(parameters, "(i&si)", &index, &text, &length);
    const auto offset = textOffset(*request.state, index, true);
    if (length < -1)
      invalid("Invalid insertion length");
    std::string value(text);
    // EditableText uses a UTF-8 byte count for the inserted string, unlike
    // document offsets.
    if (length >= 0 && static_cast<size_t>(length) < value.size()) {
      if ((static_cast<uint8_t>(value[length]) & 0xc0) == 0x80)
        invalid("Insertion length splits UTF-8 text");
      value.resize(static_cast<size_t>(length));
    }
    return g_variant_new(
        "(b)", replace(request.element, request.state, offset, offset, value));
  }
  if (!strcmp(method, "PasteText")) {
    int index;
    g_variant_get(parameters, "(i)", &index);
    const auto offset = textOffset(*request.state, index, true);
    if (!(request.state->data.actions & 32))
      unsupported();
    if (!request.element->exposed(true) || !request.host->window)
      return g_variant_new("(b)", FALSE);
    auto *pending =
        new Paste{request.host, request.element, request.state, offset,
                  G_DBUS_METHOD_INVOCATION(g_object_ref(request.invocation))};
    gdk_clipboard_read_text_async(
        gdk_display_get_clipboard(request.host->window->application->display),
        request.host->cancellable, pasteReady, pending);
    return nullptr;
  }
  int first, last;
  g_variant_get(parameters, "(ii)", &first, &last);
  const auto start = textOffset(*request.state, first, true),
             end = textOffset(*request.state, last, true);
  if (end < start)
    invalid("Text range is reversed");
  if (!strcmp(method, "DeleteText"))
    return g_variant_new(
        "(b)", replace(request.element, request.state, start, end, ""));
  if (!strcmp(method, "CopyText") || !strcmp(method, "CutText")) {
    const auto value = content.substr(static_cast<size_t>(start),
                                      static_cast<size_t>(end - start));
    bool accepted = copy(request, value);
    if (!strcmp(method, "CopyText")) {
      if (!accepted)
        throw Failure("org.a11y.atspi.Error.Failed",
                      "The clipboard rejected the text");
      return g_variant_new("()");
    }
    return g_variant_new(
        "(b)",
        accepted && replace(request.element, request.state, start, end, ""));
  }
  unsupported();
}
} // namespace swax::atspi
