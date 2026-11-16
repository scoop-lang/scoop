#include <cstdint>
#include <thread>

extern "C" int32_t scoop_graphics_test_geometry_query(
    void *context, void (*retain)(void *), void (*release)(void *),
    int32_t (*contains)(void *, double, double), double x, double y) {
  int32_t result = -1;
  std::thread worker;
  retain(context);
  try {
    worker = std::thread([&] {
      result = contains(context, x, y);
      release(context);
    });
  } catch (...) {
    release(context);
    return -1;
  }
  int32_t local = contains(context, x, y);
  worker.join();
  return local == result ? result : -1;
}
