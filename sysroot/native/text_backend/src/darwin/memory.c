#include "scoop_text_backend.h"
#include <stdlib.h>

void scoop_unicode_free(uint8_t *data, int64_t size) {
  (void)size;
  free(data);
}
