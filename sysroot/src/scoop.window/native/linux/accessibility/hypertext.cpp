#include "provider.h"

namespace swax::atspi {
GVariant *hypertext(Request &request, const char *interface, const char *method,
                    GVariant *parameters) {
  if (!strcmp(interface, "org.a11y.atspi.Hypertext")) {
    const auto &children = request.state->textChildren;
    if (!strcmp(method, "GetNLinks"))
      return g_variant_new("(i)", integer(children.size()));
    int index;
    g_variant_get(parameters, "(i)", &index);
    if (!strcmp(method, "GetLink")) {
      if (index < 0 || static_cast<size_t>(index) >= children.size())
        invalid("Text child index is out of range");
      auto owner = request.element->tree.lock();
      auto child = owner->actual(children[index].node);
      if (!child || !child->exposed())
        unavailable();
      return g_variant_new("(@(so))",
                           request.host->reference(child, request.sender));
    }
    if (!strcmp(method, "GetLinkIndex")) {
      const auto offset = textOffset(*request.state, index, true);
      for (size_t i = 0; i < children.size(); ++i)
        if (offset >= children[i].start && offset < children[i].end)
          return g_variant_new("(i)", integer(i));
      return g_variant_new("(i)", -1);
    }
  } else {
    if (!strcmp(method, "IsValid"))
      return g_variant_new("(b)", request.element->exposed());
    int index;
    g_variant_get(parameters, "(i)", &index);
    if (index != 0)
      invalid("Hyperlink anchor index is out of range");
    if (!strcmp(method, "GetObject"))
      return g_variant_new(
          "(@(so))", request.host->reference(request.element, request.sender));
    if (!strcmp(method, "GetURI"))
      return g_variant_new("(@s)", string(request.state->text[5].value_or("")));
  }
  unsupported();
}
} // namespace swax::atspi
