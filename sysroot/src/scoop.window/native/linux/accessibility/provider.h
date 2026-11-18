#ifndef SCOOP_WINDOW_LINUX_ACCESSIBILITY_H
#define SCOOP_WINDOW_LINUX_ACCESSIBILITY_H
#include "../../accessibility/core.h"
#include "../window.h"
#include <exception>
#include <stdexcept>
#include <thread>

namespace swax::atspi {
inline constexpr const char *nullPath = "/org/a11y/atspi/null";
class Failure final : public std::runtime_error {
public:
  Failure(const char *name, const char *message)
      : std::runtime_error(message), name(name) {}
  const char *name;
};
[[noreturn]] void
invalid(const char *message = "Invalid accessibility argument");
[[noreturn]] void unavailable();
[[noreturn]] void unsupported(
    const char *message = "This operation is not provided by the element");
bool actionResult(int32_t value);
int integer(int64_t value);
GVariant *string(const std::string &value);
GVariant *strings(const std::vector<std::string> &values);
GVariant *attributes(const std::map<std::string, std::string> &values);

class WindowHost final : public Host,
                         public std::enable_shared_from_this<WindowHost> {
public:
  explicit WindowHost(SWWindow *value);
  ~WindowHost();
  SWWindow *window;
  GThread *thread;
  GMainContext *context;
  std::weak_ptr<Tree> tree;
  GDBusConnection *bus = nullptr;
  GDBusNodeInfo *protocol = nullptr;
  GSource *notificationSource = nullptr;
  std::set<int64_t> pendingCollections;
  GCancellable *cancellable = nullptr;
  guint subtree = 0, cache = 0, clients = 0;
  std::string path, parentBus, parentPath;
  struct Export {
    ElementPtr element;
    std::set<std::string> clients;
  };
  std::map<uint64_t, Export> exports;
  ElementPtr root;
  bool nativeGeometryKnown = false, nativeVisible = false;
  double nativeX = 0, nativeY = 0, nativeWidth = 0, nativeHeight = 0,
         nativeScale = 0;
  bool onOwner() const override;
  int32_t dispatch(const std::function<int32_t()> &body) override;
  bool visible() const override;
  bool enabled() const override;
  bool focused() const override;
  Rect bounds() const override;
  std::optional<Rect> toScreen(Rect value) const override;
  std::optional<Point> fromScreen(Point value) const override;
  void changed(const Change &change) override;
  void scheduleCollection(int64_t number);
  void refreshCollections();
  void announce(int64_t node, const std::string &text,
                int32_t priority) override;
  void close() override;
  void attach();
  ElementPtr lookup(const char *node, const char *sender);
  std::string exportPath(const ElementPtr &element,
                         const char *sender = nullptr);
  GVariant *reference(const ElementPtr &element, const char *sender = nullptr);
  GVariant *references(const std::vector<ElementPtr> &elements,
                       const char *sender);
  GVariant *application() const;
  GVariant *parent() const;
  GVariant *cacheItem(const ElementPtr &element, const char *sender);
  void signal(const ElementPtr &element, const char *name, const char *detail,
              int a, int b, GVariant *value);
  void fail(int error);
};
struct Request {
  std::shared_ptr<WindowHost> host;
  ElementPtr element;
  std::shared_ptr<const State> state;
  const char *sender;
  GDBusMethodInvocation *invocation = nullptr;
};
struct Action {
  const char *name;
  int code;
};
std::vector<Action> actions(const State &state);
std::vector<std::string> interfaces(const ElementPtr &element,
                                    const State &state);
uint32_t role(const State &state);
const char *roleName(const State &state);
GVariant *states(const ElementPtr &element, const State &state);
std::map<std::string, std::string> objectAttributes(const State &state);
std::optional<TextChild> textChild(const ElementPtr &element,
                                   std::shared_ptr<const State> &parent);
GVariant *accessible(Request &request, const char *method,
                     GVariant *parameters);
GVariant *component(Request &request, const char *method, GVariant *parameters);
GVariant *control(Request &request, const char *interface, const char *method,
                  GVariant *parameters);
GVariant *text(Request &request, const char *method, GVariant *parameters);
GVariant *editableText(Request &request, const char *method,
                       GVariant *parameters);
GVariant *table(Request &request, const char *interface, const char *method,
                GVariant *parameters);
GVariant *hypertext(Request &request, const char *interface, const char *method,
                    GVariant *parameters);
GVariant *property(Request &request, const char *interface, const char *name);
void setProperty(Request &request, const char *interface, const char *name,
                 GVariant *value);
Rect extents(Request &request, uint32_t coordinates);
Rect convertRect(Request &request, Rect value, uint32_t coordinates);
Point localPoint(Request &request, Point value, uint32_t coordinates);
std::vector<Rect> textBoxes(const State &state, int64_t first, int64_t last);
const std::string &publicContent(const State &state);
int64_t textOffset(const State &state, int64_t index, bool allowEnd = false);
GVariant *textAttributes(const State &state);
bool selected(const State &state);
bool select(const ElementPtr &element, bool value);
ElementPtr containingTable(const ElementPtr &element);
std::vector<int> selectedRows(const ElementPtr &element);
} // namespace swax::atspi
#endif
