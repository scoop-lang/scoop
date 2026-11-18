#include "provider.h"

namespace swax::atspi {
GVariant *WindowHost::cacheItem(const ElementPtr &element, const char *sender) {
  auto state = element->state();
  if (!state)
    unavailable();
  GVariantBuilder item;
  g_variant_builder_init(&item, G_VARIANT_TYPE("((so)(so)(so)iiassusau)"));
  try {
    g_variant_builder_add_value(&item, reference(element, sender));
    g_variant_builder_add_value(&item, application());
    g_variant_builder_add_value(
        &item,
        element == root ? parent() : reference(element->parent(), sender));
    g_variant_builder_add(
        &item, "i", element == root ? 0 : integer(element->indexInParent()));
    g_variant_builder_add(&item, "i", integer(element->childCount()));
    g_variant_builder_add_value(&item, strings(interfaces(element, *state)));
    g_variant_builder_add_value(&item, string(state->text[0].value_or("")));
    g_variant_builder_add(&item, "u", role(*state));
    g_variant_builder_add_value(&item, string(state->text[1].value_or("")));
    g_variant_builder_add_value(&item, states(element, *state));
    return g_variant_builder_end(&item);
  } catch (...) {
    g_variant_builder_clear(&item);
    throw;
  }
}
static void getItems(GDBusConnection *, const char *sender, const char *,
                     const char *, const char *method, GVariant *,
                     GDBusMethodInvocation *invocation, void *data) {
  auto host = *static_cast<std::shared_ptr<WindowHost> *>(data);
  GVariantBuilder result;
  g_variant_builder_init(&result, G_VARIANT_TYPE("a((so)(so)(so)iiassusau)"));
  try {
    if (strcmp(method, "GetItems"))
      unsupported();
    // Cache only objects which clients have requested; never enumerate a
    // logical data set here.
    std::vector<ElementPtr> values;
    for (const auto &entry : host->exports)
      if (!entry.second.element->retired)
        values.push_back(entry.second.element);
    for (const auto &element : values)
      g_variant_builder_add_value(&result, host->cacheItem(element, sender));
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(@a((so)(so)(so)iiassusau))",
                                  g_variant_builder_end(&result)));
  } catch (const Failure &failure) {
    g_variant_builder_clear(&result);
    g_dbus_method_invocation_return_dbus_error(invocation, failure.name,
                                               failure.what());
  } catch (...) {
    g_variant_builder_clear(&result);
    g_dbus_method_invocation_return_dbus_error(
        invocation, "org.a11y.atspi.Error.Failed",
        "Accessibility cache unavailable");
  }
}
static GVariant *cacheProperty(GDBusConnection *, const char *, const char *,
                               const char *, const char *name, GError **error,
                               void *) {
  if (!strcmp(name, "version"))
    return g_variant_new_uint32(1);
  g_dbus_error_set_dbus_error(error,
                              "org.freedesktop.DBus.Error.UnknownProperty",
                              "Unknown cache property", nullptr);
  return nullptr;
}
static const GDBusInterfaceVTable cacheMethods = {
    getItems, cacheProperty, nullptr, {nullptr}};
static void destroyHost(void *data) {
  delete static_cast<std::shared_ptr<WindowHost> *>(data);
}
void registerCache(const std::shared_ptr<WindowHost> &host) {
  GError *error = nullptr;
  auto *data = new std::shared_ptr<WindowHost>(host);
  host->cache = g_dbus_connection_register_object(
      host->bus, "/org/a11y/atspi/cache",
      g_dbus_node_info_lookup_interface(host->protocol, "org.a11y.atspi.Cache"),
      &cacheMethods, data, destroyHost, &error);
  if (!host->cache) {
    delete data;
    g_clear_error(&error);
    throw std::runtime_error("Cannot register accessibility cache");
  }
}
} // namespace swax::atspi
