#include "protocol.h"
#include "provider.h"
#include <X11/Xlib.h>
#include <algorithm>
#include <gdk/x11/gdkx.h>
#include <gtk/a11y/gtkatspi.h>

namespace swax::atspi {
void registerProvider(const std::shared_ptr<WindowHost> &host);
WindowHost::WindowHost(SWWindow *value)
    : window(value), thread(g_thread_self()),
      context(g_main_context_ref(value->application->main_context)),
      cancellable(g_cancellable_new()) {}
WindowHost::~WindowHost() {
  g_clear_object(&cancellable);
  if (protocol)
    g_dbus_node_info_unref(protocol);
  if (bus)
    g_object_unref(bus);
  g_main_context_unref(context);
}
bool WindowHost::onOwner() const { return g_thread_self() == thread; }
int32_t WindowHost::dispatch(const std::function<int32_t()> &body) {
  return onOwner() ? body() : 6;
}
bool WindowHost::visible() const {
  return window && !window->destroying && gtk_widget_get_mapped(window->view) &&
         !window->snapshot.minimized;
}
bool WindowHost::enabled() const {
  return window && !window->destroying && !window->input_blocked &&
         gtk_widget_is_sensitive(window->view);
}
bool WindowHost::focused() const {
  return window && !window->destroying &&
         gtk_window_is_active(GTK_WINDOW(window->window));
}
Rect WindowHost::bounds() const {
  return window ? Rect{0, 0,
                       static_cast<double>(gtk_widget_get_width(window->view)),
                       static_cast<double>(gtk_widget_get_height(window->view))}
                : Rect{};
}
std::optional<Rect> WindowHost::toScreen(Rect value) const {
  if (!window || window->destroying)
    return {};
  auto *surface = sw_surface(window);
  if (!surface || !GDK_IS_X11_SURFACE(surface))
    return {};
  graphene_point_t zero = GRAPHENE_POINT_INIT(0, 0), offset{};
  if (!gtk_widget_compute_point(window->view, window->window, &zero, &offset))
    return {};
  Display *display = gdk_x11_display_get_xdisplay(window->application->display);
  ::Window child = 0;
  int x = 0, y = 0;
  gdk_x11_display_error_trap_push(window->application->display);
  bool success =
      XTranslateCoordinates(display, gdk_x11_surface_get_xid(surface),
                            DefaultRootWindow(display), 0, 0, &x, &y, &child);
  if (gdk_x11_display_error_trap_pop(window->application->display) || !success)
    return {};
  double scale = gdk_surface_get_scale(surface);
  return Rect{value.x + offset.x + x / scale, value.y + offset.y + y / scale,
              value.width, value.height};
}
std::optional<Point> WindowHost::fromScreen(Point value) const {
  auto origin = toScreen({});
  return origin
             ? std::optional<Point>({value.x - origin->x, value.y - origin->y})
             : std::nullopt;
}
void WindowHost::fail(int error) {
  if (window && !window->destroying)
    sw_fail(window->application, error);
}
void WindowHost::attach() {
  auto owner = tree.lock();
  path = "/org/scoop/a11y/window_" + std::to_string(owner->identity);
  root = owner->actual(0);
  exports[root->serial] = {root, {}};
  // A disabled desktop accessibility service must not prevent the window from
  // opening.
  if (g_strcmp0(g_getenv("GTK_A11Y"), "none") == 0)
    return;
  GError *error = nullptr;
  auto *session = g_bus_get_sync(G_BUS_TYPE_SESSION, cancellable, &error);
  if (!session) {
    const bool absent =
        g_error_matches(error, G_IO_ERROR, G_IO_ERROR_NOT_FOUND) ||
        g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CONNECTION_REFUSED) ||
        g_error_matches(error, G_DBUS_ERROR, G_DBUS_ERROR_NO_SERVER) ||
        g_error_matches(error, G_DBUS_ERROR, G_DBUS_ERROR_FILE_NOT_FOUND);
    g_clear_error(&error);
    if (absent)
      return;
    throw std::runtime_error("Cannot connect to the session bus");
  }
  auto *reply = g_dbus_connection_call_sync(
      session, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus", "GetAddress",
      nullptr, G_VARIANT_TYPE("(s)"), G_DBUS_CALL_FLAGS_NONE, -1, cancellable,
      &error);
  g_object_unref(session);
  if (!reply) {
    bool absent =
        g_error_matches(error, G_DBUS_ERROR, G_DBUS_ERROR_SERVICE_UNKNOWN) ||
        g_error_matches(error, G_DBUS_ERROR, G_DBUS_ERROR_NAME_HAS_NO_OWNER);
    g_clear_error(&error);
    if (absent)
      return;
    throw std::runtime_error("Cannot obtain the accessibility bus address");
  }
  const char *address = nullptr;
  g_variant_get(reply, "(&s)", &address);
  bus = g_dbus_connection_new_for_address_sync(
      address,
      static_cast<GDBusConnectionFlags>(
          G_DBUS_CONNECTION_FLAGS_AUTHENTICATION_CLIENT |
          G_DBUS_CONNECTION_FLAGS_MESSAGE_BUS_CONNECTION),
      nullptr, cancellable, &error);
  g_variant_unref(reply);
  if (!bus) {
    g_clear_error(&error);
    throw std::runtime_error("Cannot connect to the accessibility bus");
  }
  g_dbus_connection_set_exit_on_close(bus, FALSE);
  protocol = g_dbus_node_info_new_for_xml(protocolXml, &error);
  if (!protocol) {
    g_clear_error(&error);
    throw std::runtime_error("Invalid accessibility protocol declaration");
  }
  registerProvider(shared_from_this());
  auto *socket = gtk_at_spi_socket_new(g_dbus_connection_get_unique_name(bus),
                                       path.c_str(), &error);
  if (!socket) {
    g_clear_error(&error);
    throw std::runtime_error("Cannot attach the GTK accessibility socket");
  }
  window->accessibility_socket = socket;
  auto *child = gtk_widget_get_first_child(window->view);
  gtk_accessible_set_accessible_parent(socket, GTK_ACCESSIBLE(window->view),
                                       child ? GTK_ACCESSIBLE(child) : nullptr);
}
ElementPtr WindowHost::lookup(const char *node, const char *sender) {
  auto owner = tree.lock();
  if (!owner || owner->closed)
    return {};
  if (!node || !*node) {
    if (sender && *sender)
      exports[root->serial].clients.insert(sender);
    return root;
  }
  if (*node != 'e' || !node[1])
    return {};
  char *end = nullptr;
  errno = 0;
  auto serial = g_ascii_strtoull(node + 1, &end, 10);
  if (errno || *end)
    return {};
  auto found = exports.find(serial);
  if (found == exports.end() || found->second.element->retired ||
      !found->second.element->state())
    return {};
  if (sender && *sender)
    found->second.clients.insert(sender);
  return found->second.element;
}
std::string WindowHost::exportPath(const ElementPtr &element,
                                   const char *sender) {
  if (!element)
    return nullPath;
  auto &value = exports[element->serial];
  value.element = element;
  if (sender && *sender)
    value.clients.insert(sender);
  return element == root ? path : path + "/e" + std::to_string(element->serial);
}
GVariant *WindowHost::reference(const ElementPtr &element, const char *sender) {
  if (!element || !bus)
    return g_variant_new("(so)", "", nullPath);
  return g_variant_new("(so)", g_dbus_connection_get_unique_name(bus),
                       exportPath(element, sender).c_str());
}
GVariant *WindowHost::references(const std::vector<ElementPtr> &elements,
                                 const char *sender) {
  GVariantBuilder values;
  g_variant_builder_init(&values, G_VARIANT_TYPE("a(so)"));
  for (const auto &element : elements)
    g_variant_builder_add_value(&values, reference(element, sender));
  return g_variant_builder_end(&values);
}
GVariant *WindowHost::application() const {
  return parentBus.empty() ? g_variant_new("(so)", "", nullPath)
                           : g_variant_new("(so)", parentBus.c_str(),
                                           "/org/a11y/atspi/accessible/root");
}
GVariant *WindowHost::parent() const {
  return parentBus.empty()
             ? g_variant_new("(so)", "", nullPath)
             : g_variant_new("(so)", parentBus.c_str(), parentPath.c_str());
}
void WindowHost::close() {
  if (notificationSource) {
    g_source_destroy(notificationSource);
    g_source_unref(notificationSource);
    notificationSource = nullptr;
  }
  pendingCollections.clear();
  if (cancellable)
    g_cancellable_cancel(cancellable);
  if (window) {
    if (window->accessibility_socket) {
      gtk_accessible_set_accessible_parent(window->accessibility_socket,
                                           nullptr, nullptr);
      g_clear_object(&window->accessibility_socket);
    }
    window->accessibility = nullptr;
  }
  if (bus) {
    if (subtree) {
      g_dbus_connection_unregister_subtree(bus, subtree);
      subtree = 0;
    }
    if (cache) {
      g_dbus_connection_unregister_object(bus, cache);
      cache = 0;
    }
    if (clients) {
      g_dbus_connection_signal_unsubscribe(bus, clients);
      clients = 0;
    }
    g_dbus_connection_close(bus, nullptr, nullptr, nullptr);
    g_clear_object(&bus);
  }
  exports.clear();
  root.reset();
  window = nullptr;
}
} // namespace swax::atspi
extern "C" SWResult scoop_window_accessibility_create(
    SWWindow *window, SWAccessibilityCallback callback, void *context) {
  if (!window || window->destroying || window->accessibility)
    return {nullptr, -4};
  std::shared_ptr<swax::atspi::WindowHost> host;
  try {
    host = std::make_shared<swax::atspi::WindowHost>(window);
    auto tree = std::make_shared<swax::Tree>(host, callback, context);
    host->tree = tree;
    auto handle = std::make_unique<swax::Handle>();
    handle->tree = tree;
    window->accessibility = host.get();
    host->attach();
    return {handle.release(), 0};
  } catch (const std::bad_alloc &) {
    if (host)
      host->close();
    return {nullptr, ENOMEM};
  } catch (...) {
    if (host)
      host->close();
    return {nullptr, EIO};
  }
}
extern "C" void sw_accessibility_destroy(SWWindow *window) {
  auto *host = static_cast<swax::atspi::WindowHost *>(window->accessibility);
  if (host) {
    if (auto tree = host->tree.lock())
      tree->close();
  }
}
