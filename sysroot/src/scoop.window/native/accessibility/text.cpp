#include "core.h"
#include <stdexcept>

namespace swax {
bool publicText(const State &state) {
  return !(state.data.flags & SWAXProtected) &&
         (state.data.role == 1 || state.data.role == 4) && state.text[3];
}
static int width(uint8_t byte) {
  if (byte < 0x80)
    return 1;
  if ((byte & 0xe0) == 0xc0)
    return 2;
  if ((byte & 0xf0) == 0xe0)
    return 3;
  if ((byte & 0xf8) == 0xf0)
    return 4;
  return 0;
}
int64_t utf8Offset(const std::string &text, int64_t units, bool utf16) {
  if (units < 0)
    return -1;
  int64_t index = 0, count = 0;
  while (index < static_cast<int64_t>(text.size()) && count < units) {
    int size = width(static_cast<uint8_t>(text[index]));
    if (!size || index + size > static_cast<int64_t>(text.size()))
      return -1;
    count += utf16 && size == 4 ? 2 : 1;
    index += size;
  }
  return count == units ? index : -1;
}
int64_t textUnits(const std::string &text, int64_t bytes, bool utf16) {
  if (bytes < 0 || bytes > static_cast<int64_t>(text.size()))
    return -1;
  int64_t index = 0, count = 0;
  while (index < bytes) {
    int size = width(static_cast<uint8_t>(text[index]));
    if (!size || index + size > bytes)
      return -1;
    count += utf16 && size == 4 ? 2 : 1;
    index += size;
  }
  return count;
}
std::string textSlice(const std::string &text, int64_t start, int64_t end,
                      bool utf16) {
  const auto first = utf8Offset(text, start, utf16);
  const auto last = end == -1 ? static_cast<int64_t>(text.size())
                              : utf8Offset(text, end, utf16);
  if (first < 0 || last < first)
    throw std::invalid_argument("invalid text range");
  return text.substr(static_cast<size_t>(first),
                     static_cast<size_t>(last - first));
}
} // namespace swax
