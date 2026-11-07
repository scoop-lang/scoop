#ifndef SCOOP_TRAY_H
#define SCOOP_TRAY_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
  int32_t kind, action, source, clicks;
  int64_t button;
  int32_t anchor, reserved;
  double x, y, width, height;
  int64_t error;
} STEvent;
typedef int32_t (*STCallback)(void *, const STEvent *);
typedef struct {
  void *pointer;
  int64_t error;
} STResult;
typedef struct {
  int64_t error;
  double x, y, width, height;
} STGeometry;
typedef struct {
  int32_t command;
  int64_t error;
} STSelection;

#endif
