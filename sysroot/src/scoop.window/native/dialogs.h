#ifndef SCOOP_WINDOW_DIALOGS_H
#define SCOOP_WINDOW_DIALOGS_H

#include <stdint.h>

typedef struct {
  int32_t outcome;
  int64_t error, count;
  const uint8_t *domain;
  int64_t domain_length;
} SWFileResult;
typedef struct {
  const uint8_t *data;
  int64_t length;
} SWFilePath;

#endif
