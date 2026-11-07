#include <stdint.h>
#include <string.h>

typedef struct {
  const uint8_t *data;
  int64_t length;
} NativePath;

#ifdef _WIN32
static const uint16_t path[] = {'C', ':', '\\', 0xd800, '.', 't', 'x', 't'};
int32_t scoop_test_file_path_windows(void) { return 1; }
#else
static const uint8_t path[] = {'/',  't', 'm', 'p', '/',
                               0xff, '.', 't', 'x', 't'};
int32_t scoop_test_file_path_windows(void) { return 0; }
#endif

NativePath scoop_test_file_path_result(void) {
  return (NativePath){(const uint8_t *)path, sizeof(path)};
}
int32_t scoop_test_file_path_option(const uint8_t *data, int64_t length) {
  return length == (int64_t)sizeof(path) &&
         memcmp(data, path, sizeof(path)) == 0;
}
