#include <stdexcept>
#include <vector>
extern "C" int native_answer(void) {
  try {
    throw std::runtime_error("native");
  } catch (const std::runtime_error &) {
    return std::vector<int>{42}[0];
  }
}
