#include "text_services.h"
#include <cerrno>
#include <new>

extern "C" int64_t
scoop_window_text_service_count(SWTextServiceResults *value) {
  return static_cast<int64_t>(value->rows.size());
}
extern "C" SWTextServiceItem
scoop_window_text_service_item(SWTextServiceResults *value, int64_t index) {
  const auto &row = value->rows.at(static_cast<size_t>(index));
  return {row.start, row.end, row.kind,
          static_cast<int64_t>(row.suggestions.size())};
}
extern "C" SWTextServiceString
scoop_window_text_service_string(SWTextServiceResults *value, int64_t index,
                                 int64_t suggestion) {
  const auto &row = value->rows.at(static_cast<size_t>(index));
  const auto &text = suggestion < 0
                         ? row.description
                         : row.suggestions.at(static_cast<size_t>(suggestion));
  return {reinterpret_cast<const uint8_t *>(text.data()),
          static_cast<int64_t>(text.size())};
}
extern "C" void scoop_window_text_service_destroy(SWTextServiceResults *value) {
  delete value;
}

#if !defined(__APPLE__) && !defined(_WIN32)
extern "C" int32_t scoop_window_text_service_flags() { return 0; }
extern "C" SWTextServiceResult scoop_window_text_check(const uint8_t *, int64_t,
                                                       int32_t, const uint8_t *,
                                                       int64_t) {
  auto *value = new (std::nothrow) SWTextServiceResults;
  return {value, value ? 0 : ENOMEM};
}
extern "C" SWTextServiceResult scoop_window_text_services(void *) {
  return scoop_window_text_check(nullptr, 0, 0, nullptr, 0);
}
extern "C" SWTextServiceOutput
scoop_window_text_service_perform(const uint8_t *, int64_t, const uint8_t *,
                                  int64_t) {
  return {};
}
extern "C" int64_t scoop_window_text_definition(void *, const uint8_t *,
                                                int64_t, double, double, double,
                                                double) {
  return 0;
}
#endif
