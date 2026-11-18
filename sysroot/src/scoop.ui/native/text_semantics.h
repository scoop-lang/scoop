#pragma once

#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstdint>
#include <vector>

struct UITextPoint {
  double x, y;
};
struct UITextRect {
  double x, y, width, height;
};
struct UITextRange {
  int64_t start, end;
};
struct UITextPosition {
  int64_t byte;
  int32_t affinity, status;
};
struct UITextCaret {
  UITextRect rect;
  int32_t status;
};
struct UITextRangeResult {
  UITextRange range;
  int32_t status;
};
struct UITextPointResult {
  int32_t status;
  UITextPoint point;
};
struct UITextRectResult {
  int32_t status;
  UITextRect rect;
};
using UITextBoxVisitor = void (*)(void *, UITextRect, int32_t);

// These value layouts are the public graphics and Window native query
// contracts.
struct UITextLayout {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  UITextPosition (*hit)(void *, UITextPoint);
  UITextCaret (*caret)(void *, int64_t, int32_t, double);
  int32_t (*boxes)(void *, int64_t, int64_t, void *, UITextBoxVisitor);
  UITextRangeResult (*range)(void *, int64_t, int32_t, int32_t);
  UITextRangeResult (*line_range)(void *, int64_t);
  int64_t (*line_index)(void *, int64_t, int32_t);
};

struct UITextSceneGeometry {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  int32_t (*contains)(void *, double, double);
  UITextPointResult (*to_local)(void *, double, double);
  UITextRectResult (*map_rect)(void *, UITextRect, int32_t);
};

struct UITextMapping {
  int64_t source_start, source_end, display_start, display_end;
};
struct UITextLine {
  int64_t start, end, visible_start, visible_end;
};
struct UITextCell {
  int64_t start, end;
  UITextRect bounds, caret;
  double hit_x;
};
struct UITextSourceData {
  int64_t source_length, display_length;
  int32_t cells, expand;
  double caret_width, caret_aspect;
  const int64_t *graphemes;
  int64_t grapheme_count;
  const UITextMapping *mapping;
  int64_t mapping_count;
  const UITextLine *lines;
  int64_t line_count;
  const UITextCell *cell_values;
  int64_t cell_count;
};

struct UITextSource {
  std::atomic<uint64_t> references{1};
  UITextLayout layout;
  int64_t source_length, display_length;
  bool cells, expand;
  double caret_width, caret_aspect;
  std::vector<int64_t> graphemes;
  std::vector<UITextMapping> mapping;
  std::vector<UITextLine> lines;
  std::vector<UITextCell> cell_values;

  UITextSource(UITextLayout layout, const UITextSourceData &data);
  ~UITextSource();
  void retain();
  void release();
  int64_t map(int64_t offset, int32_t affinity, bool backwards) const;
  int64_t visual(int64_t offset, int32_t affinity) const;
  UITextRangeResult source_range(UITextRange range) const;
  UITextRangeResult unit(int64_t offset, int32_t affinity, int32_t unit) const;
  UITextRangeResult line_range(int64_t index) const;
  int64_t line_index(int64_t offset, int32_t affinity) const;
  UITextPosition hit(UITextPoint point) const;
  UITextCaret caret(int64_t offset, int32_t affinity) const;
  int32_t boxes(int64_t start, int64_t end, void *context,
                UITextBoxVisitor visitor) const;
};

using UISemanticBoxVisitor = void (*)(void *, double, double, double, double);
struct UISemanticTextGeometry {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  UITextRangeResult (*range)(void *, int64_t, int32_t, int32_t);
  UITextRangeResult (*line_range)(void *, int64_t);
  int64_t (*line_index)(void *, int64_t, int32_t);
  UITextRangeResult (*range_at_point)(void *, double, double);
  int32_t (*boxes)(void *, int64_t, int64_t, void *, UISemanticBoxVisitor);
  UITextRangeResult (*visible_range)(void *);
};
