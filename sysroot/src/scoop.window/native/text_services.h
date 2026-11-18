#ifndef SCOOP_WINDOW_TEXT_SERVICES_H
#define SCOOP_WINDOW_TEXT_SERVICES_H

#include <stdint.h>
#ifdef __cplusplus
#include <string>
#include <vector>
#endif

typedef struct SWTextServiceResult {
  void *value;
  int64_t error;
} SWTextServiceResult;
typedef struct SWTextServiceString {
  const uint8_t *bytes;
  int64_t length;
} SWTextServiceString;
typedef struct SWTextServiceOutput {
  uint8_t *bytes;
  int64_t length, error;
} SWTextServiceOutput;
typedef struct SWTextServiceItem {
  int64_t start, end;
  int32_t kind;
  int64_t suggestions;
} SWTextServiceItem;
#ifdef __cplusplus
struct SWTextServiceRow {
  int64_t start = 0, end = 0;
  int32_t kind = 0;
  std::string description;
  std::vector<std::string> suggestions;
};
struct SWTextServiceResults {
  std::vector<SWTextServiceRow> rows;
};
#endif

#endif
