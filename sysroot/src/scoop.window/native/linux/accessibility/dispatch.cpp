#include "provider.h"
#include <algorithm>

namespace swax::atspi {
[[noreturn]] void invalid(const char *message) {
  throw Failure("org.freedesktop.DBus.Error.InvalidArgs", message);
}
[[noreturn]] void unavailable() {
  throw Failure("org.a11y.atspi.Error.Defunct",
                "The element or its published layout is unavailable");
}
[[noreturn]] void unsupported(const char *message) {
  throw Failure("org.a11y.atspi.Error.NotSupported", message);
}
bool actionResult(int32_t value) {
  switch (value) {
  case 1:
    return true;
  case 2:
    invalid();
  case 3:
    unsupported();
  case 4:
    unavailable();
  case 5:
    return false;
  case 6:
    throw Failure("org.a11y.atspi.Error.Failed",
                  "The owner is busy; retry after the next published state");
  default:
    throw Failure("org.a11y.atspi.Error.Failed",
                  "Accessibility operation failed");
  }
}
int integer(int64_t value) {
  if (value < INT_MIN || value > INT_MAX)
    unsupported("The value exceeds the AT-SPI integer range");
  return static_cast<int>(value);
}
GVariant *string(const std::string &value) {
  if (value.find('\0') != std::string::npos)
    unsupported("D-Bus strings cannot represent embedded NUL characters");
  return g_variant_new_string(value.c_str());
}
GVariant *strings(const std::vector<std::string> &values) {
  for (const auto &value : values)
    if (value.find('\0') != std::string::npos)
      unsupported("D-Bus strings cannot represent embedded NUL characters");
  GVariantBuilder result;
  g_variant_builder_init(&result, G_VARIANT_TYPE("as"));
  for (const auto &value : values)
    g_variant_builder_add_value(&result, string(value));
  return g_variant_builder_end(&result);
}
GVariant *attributes(const std::map<std::string, std::string> &values) {
  for (const auto &value : values)
    if (value.first.find('\0') != std::string::npos ||
        value.second.find('\0') != std::string::npos)
      unsupported("D-Bus strings cannot represent embedded NUL characters");
  GVariantBuilder result;
  g_variant_builder_init(&result, G_VARIANT_TYPE("a{ss}"));
  for (const auto &value : values) {
    auto key = string(value.first), data = string(value.second);
    g_variant_builder_add_value(&result, g_variant_new_dict_entry(key, data));
  }
  return g_variant_builder_end(&result);
}
static const char *nodeName(const WindowHost &host, const char *path) {
  if (host.path == path)
    return nullptr;
  if (strncmp(path, host.path.c_str(), host.path.size()) ||
      path[host.path.size()] != '/')
    unavailable();
  return path + host.path.size() + 1;
}
static Request request(void *data, const char *sender, const char *path,
                       GDBusMethodInvocation *invocation = nullptr) {
  auto host = *static_cast<std::shared_ptr<WindowHost> *>(data);
  auto element = host->lookup(nodeName(*host, path), sender);
  if (!element)
    unavailable();
  auto state = element->state();
  if (!state)
    unavailable();
  return {std::move(host), std::move(element), std::move(state), sender,
          invocation};
}
static void methodCall(GDBusConnection *, const char *sender, const char *path,
                       const char *interface, const char *name,
                       GVariant *parameters, GDBusMethodInvocation *invocation,
                       void *data) {
  try {
    auto input = request(data, sender, path, invocation);
    GVariant *reply = nullptr;
    if (!strcmp(interface, "org.a11y.atspi.Accessible"))
      reply = accessible(input, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.Component"))
      reply = component(input, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.Action") ||
             !strcmp(interface, "org.a11y.atspi.Selection"))
      reply = control(input, interface, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.Text"))
      reply = text(input, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.EditableText"))
      reply = editableText(input, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.Table") ||
             !strcmp(interface, "org.a11y.atspi.TableCell"))
      reply = table(input, interface, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.Hypertext") ||
             !strcmp(interface, "org.a11y.atspi.Hyperlink"))
      reply = hypertext(input, interface, name, parameters);
    else if (!strcmp(interface, "org.a11y.atspi.Socket") &&
             input.element == input.host->root && !strcmp(name, "Embedded")) {
      const char *parent = nullptr;
      g_variant_get(parameters, "(&s)", &parent);
      if (!g_variant_is_object_path(parent))
        invalid("Invalid embedding parent path");
      input.host->parentBus = sender;
      input.host->parentPath = parent;
      reply = g_variant_new("()");
    } else
      unsupported();
    // Asynchronous clipboard reads own their invocation until the normal main
    // context resumes.
    if (reply)
      g_dbus_method_invocation_return_value(invocation, reply);
  } catch (const Failure &error) {
    g_dbus_method_invocation_return_dbus_error(invocation, error.name,
                                               error.what());
  } catch (const std::bad_alloc &) {
    g_dbus_method_invocation_return_dbus_error(
        invocation, "org.freedesktop.DBus.Error.NoMemory",
        "Accessibility allocation failed");
  } catch (...) {
    g_dbus_method_invocation_return_dbus_error(invocation,
                                               "org.a11y.atspi.Error.Failed",
                                               "Accessibility request failed");
  }
}
static GVariant *getProperty(GDBusConnection *, const char *sender,
                             const char *path, const char *interface,
                             const char *name, GError **error, void *data) {
  try {
    auto input = request(data, sender, path);
    return property(input, interface, name);
  } catch (const Failure &failure) {
    g_dbus_error_set_dbus_error(error, failure.name, failure.what(), nullptr);
  } catch (...) {
    g_dbus_error_set_dbus_error(error, "org.a11y.atspi.Error.Failed",
                                "Accessibility property unavailable", nullptr);
  }
  return nullptr;
}
static gboolean setPropertyCall(GDBusConnection *, const char *sender,
                                const char *path, const char *interface,
                                const char *name, GVariant *value,
                                GError **error, void *data) {
  try {
    auto input = request(data, sender, path);
    setProperty(input, interface, name, value);
    return TRUE;
  } catch (const Failure &failure) {
    g_dbus_error_set_dbus_error(error, failure.name, failure.what(), nullptr);
  } catch (...) {
    g_dbus_error_set_dbus_error(error, "org.a11y.atspi.Error.Failed",
                                "Accessibility property write failed", nullptr);
  }
  return FALSE;
}
static const GDBusInterfaceVTable methods = {
    methodCall, getProperty, setPropertyCall, {nullptr}};
static gchar **enumerate(GDBusConnection *, const char *, const char *,
                         void *data) {
  const auto &host = *static_cast<std::shared_ptr<WindowHost> *>(data);
  auto **result = g_new0(gchar *, host->exports.size() + 1);
  try {
    size_t index = 0;
    for (const auto &entry : host->exports)
      if (entry.second.element != host->root && !entry.second.element->retired)
        result[index++] = g_strdup(("e" + std::to_string(entry.first)).c_str());
    return result;
  } catch (...) {
    g_strfreev(result);
    host->fail(ENOMEM);
    return nullptr;
  }
}
static GDBusInterfaceInfo **introspect(GDBusConnection *, const char *sender,
                                       const char *, const char *node,
                                       void *data) {
  try {
    auto host = *static_cast<std::shared_ptr<WindowHost> *>(data);
    auto element = host->lookup(node, sender);
    auto state = element ? element->state() : nullptr;
    if (!state)
      return nullptr;
    auto names = interfaces(element, *state);
    auto **result = g_new0(GDBusInterfaceInfo *, names.size() + 1);
    for (size_t index = 0; index < names.size(); ++index)
      result[index] =
          g_dbus_interface_info_ref(g_dbus_node_info_lookup_interface(
              host->protocol, names[index].c_str()));
    return result;
  } catch (...) {
    return nullptr;
  }
}
static const GDBusInterfaceVTable *
dispatch(GDBusConnection *, const char *sender, const char *,
         const char *interface, const char *node, void **out, void *data) {
  try {
    auto host = *static_cast<std::shared_ptr<WindowHost> *>(data);
    auto element = host->lookup(node, sender);
    auto state = element ? element->state() : nullptr;
    if (!state)
      return nullptr;
    auto names = interfaces(element, *state);
    if (std::find(names.begin(), names.end(), interface) == names.end())
      return nullptr;
    *out = data;
    return &methods;
  } catch (...) {
    return nullptr;
  }
}
static const GDBusSubtreeVTable subtreeMethods = {
    enumerate, introspect, dispatch, {nullptr}};
static void destroyHost(void *data) {
  delete static_cast<std::shared_ptr<WindowHost> *>(data);
}
static void clientChanged(GDBusConnection *, const char *, const char *,
                          const char *, const char *, GVariant *parameters,
                          void *data) {
  auto host = *static_cast<std::shared_ptr<WindowHost> *>(data);
  const char *name = nullptr, *before = nullptr, *after = nullptr;
  g_variant_get(parameters, "(&s&s&s)", &name, &before, &after);
  if (!*before || *after || *name != ':')
    return;
  try {
    for (auto it = host->exports.begin(); it != host->exports.end();) {
      auto &entry = it->second;
      bool owned = entry.clients.erase(name) != 0;
      if (owned && entry.clients.empty() && entry.element != host->root)
        it = host->exports.erase(it);
      else
        ++it;
    }
  } catch (...) {
    host->fail(ENOMEM);
  }
}
void registerCache(const std::shared_ptr<WindowHost> &host);
void registerProvider(const std::shared_ptr<WindowHost> &host) {
  GError *error = nullptr;
  auto *data = new std::shared_ptr<WindowHost>(host);
  host->subtree = g_dbus_connection_register_subtree(
      host->bus, host->path.c_str(), &subtreeMethods,
      G_DBUS_SUBTREE_FLAGS_DISPATCH_TO_UNENUMERATED_NODES, data, destroyHost,
      &error);
  if (!host->subtree) {
    delete data;
    g_clear_error(&error);
    throw std::runtime_error("Cannot register accessibility objects");
  }
  host->clients = g_dbus_connection_signal_subscribe(
      host->bus, "org.freedesktop.DBus", "org.freedesktop.DBus",
      "NameOwnerChanged", "/org/freedesktop/DBus", nullptr,
      G_DBUS_SIGNAL_FLAGS_NONE, clientChanged,
      new std::shared_ptr<WindowHost>(host), destroyHost);
  registerCache(host);
}
} // namespace swax::atspi
