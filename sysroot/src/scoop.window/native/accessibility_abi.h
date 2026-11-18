#ifndef SCOOP_WINDOW_ACCESSIBILITY_ABI_H
#define SCOOP_WINDOW_ACCESSIBILITY_ABI_H

#include <stdint.h>

typedef struct {
  const uint8_t *bytes;
  int64_t length;
} SWAccessibilityBytes;

typedef int32_t (*SWAccessibilityCallback)(void *, int64_t, int32_t,
                                           const uint8_t *, int64_t, int64_t,
                                           int64_t, uint64_t, double);
typedef int32_t (*SWAccessibilityCollectionCallback)(void *, int64_t, int32_t,
                                                     int64_t, int32_t, int64_t,
                                                     const uint8_t *, int64_t,
                                                     void *);

typedef struct {
  int64_t number, parent, order, scene_order;
  uint64_t content_revision, text_revision;
  int64_t anchor, cursor;
  int32_t role, flags, actions, checked, geometry, visible, scroll_orientation;
  double x, y, width, height;
  double visible_x, visible_y, visible_width, visible_height;
  double scroll_offset, scroll_viewport, scroll_content;
  int32_t selected, expanded, heading_level, has_range, live;
  double minimum, maximum, number_value, number_step;
  int64_t table_rows, table_columns, cell_row, cell_column, row_span,
      column_span;
  int32_t activation;
  double activation_x, activation_y;
} SWAccessibilityData;

typedef struct {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  int32_t (*contains)(void *, double, double);
} SWAccessibilityHitData;

typedef struct {
  int64_t start, end;
  int32_t status;
} SWAccessibilityTextRange;

typedef void (*SWAccessibilityTextBoxVisitor)(void *, double, double, double,
                                              double);

typedef struct {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  SWAccessibilityTextRange (*range)(void *, int64_t, int32_t, int32_t);
  SWAccessibilityTextRange (*line_range)(void *, int64_t);
  int64_t (*line_index)(void *, int64_t, int32_t);
  SWAccessibilityTextRange (*range_at_point)(void *, double, double);
  int32_t (*boxes)(void *, int64_t, int64_t, void *,
                   SWAccessibilityTextBoxVisitor);
  SWAccessibilityTextRange (*visible_range)(void *);
} SWAccessibilityTextData;

enum {
  SWAXEnabled = 1,
  SWAXReadOnly = 2,
  SWAXProtected = 4,
  SWAXHidden = 8,
  SWAXNamed = 16,
  SWAXIgnored = 32,
  SWAXRequired = 64,
  SWAXCursorUpstream = 128,
  SWAXInvalid = 256,
  SWAXBusy = 512,
  SWAXModal = 1024
};

#endif
