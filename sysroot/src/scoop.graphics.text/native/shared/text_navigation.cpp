#include "text.h"

void STTextLayout::find_carets() {
  if (carets_known)
    return;
  std::vector<std::vector<STVisualCaret>> result(lines.size());
  auto append = [&](int64_t byte, int32_t affinity, bool right_edge) {
    auto value = scoop_graphics_text_caret(this, byte, affinity, 0);
    if (value.status < 0)
      throw std::runtime_error("caret geometry failed");
    if (!value.status)
      return;
    int line = line_at((size_t)byte, affinity == 0);
    if (line < 0 || !std::isfinite(value.rect.x))
      throw std::runtime_error("invalid caret geometry");
    result[(size_t)line].push_back({byte, affinity, value.rect.x, right_edge});
  };
  for (size_t index = 1; index < data->graphemes.size(); ++index) {
    int64_t start = data->graphemes[index - 1];
    int64_t end = data->graphemes[index];
    if (info.exceeded && (size_t)start > visible_end())
      break;
    sgt::Paragraph::GlyphInfo glyph;
    bool ltr = style.getTextDirection() == sgt::TextDirection::kLtr;
    if (data->hard_break(start, end)) {
      append(start, 1, ltr);
      continue;
    }
    if (paragraph->getGlyphInfoAtUTF16Offset(data->to16[(size_t)start], &glyph))
      ltr = glyph.fDirection == sgt::TextDirection::kLtr;
    append(start, 1, !ltr);
    append(end, 0, ltr);
  }
  append((int64_t)data->text.size(), 1,
         style.getTextDirection() == sgt::TextDirection::kLtr);
  for (auto &line : result) {
    std::stable_sort(line.begin(), line.end(),
                     [](const STVisualCaret &a, const STVisualCaret &b) {
                       return a.x < b.x;
                     });
  }
  carets = std::move(result);
  carets_known = true;
}

namespace {
const STVisualCaret &at_x(const std::vector<STVisualCaret> &line, double x,
                          bool right_edge) {
  auto first =
      std::lower_bound(line.begin(), line.end(), x,
                       [](const STVisualCaret &value, double position) {
                         return value.x < position;
                       });
  for (auto value = first; value != line.end() && value->x == x; ++value)
    if (value->right_edge == right_edge)
      return *value;
  return *first;
}

const STVisualCaret &edge(const std::vector<STVisualCaret> &line, bool right) {
  return at_x(line, right ? line.back().x : line.front().x, right);
}

const STVisualCaret &nearest(const std::vector<STVisualCaret> &line, double x) {
  auto next = std::lower_bound(line.begin(), line.end(), x,
                               [](const STVisualCaret &value, double position) {
                                 return value.x < position;
                               });
  double chosen;
  if (next == line.end())
    chosen = line.back().x;
  else if (next == line.begin())
    chosen = next->x;
  else
    chosen = next->x - x < x - (next - 1)->x ? next->x : (next - 1)->x;
  return at_x(line, chosen, x >= chosen);
}

STCaretMove located(STTextLayout *layout, int64_t byte, int32_t affinity) {
  auto caret = scoop_graphics_text_caret(layout, byte, affinity, 0);
  return {byte, caret.rect.x, affinity, caret.status};
}

STCaretMove selected(const STVisualCaret &value, double preferred_x) {
  return {value.byte, preferred_x, value.affinity, 1};
}

struct STHorizontalStep {
  STCaretMove move;
  bool crossed_glyph;
};

STHorizontalStep horizontal(STTextLayout *layout, STCaretMove current,
                            int line_index, bool right) {
  const auto &line = layout->carets[(size_t)line_index];
  if (line.empty())
    return {{0, 0, 1, 0}, false};
  if (right) {
    auto next = std::upper_bound(
        line.begin(), line.end(), current.preferred_x,
        [](double x, const STVisualCaret &value) { return x < value.x; });
    if (next != line.end())
      return {selected(at_x(line, next->x, true), next->x), true};
  } else {
    auto next = std::lower_bound(
        line.begin(), line.end(), current.preferred_x,
        [](const STVisualCaret &value, double x) { return value.x < x; });
    if (next != line.begin()) {
      double x = (next - 1)->x;
      return {selected(at_x(line, x, false), x), true};
    }
  }
  bool ltr = layout->style.getTextDirection() == sgt::TextDirection::kLtr;
  int next_line = line_index + (right == ltr ? 1 : -1);
  if (next_line < 0 || next_line >= (int)layout->lines.size())
    return {current, false};
  const auto &target = layout->carets[(size_t)next_line];
  if (target.empty())
    return {{0, 0, 1, 0}, false};
  const auto &value = edge(target, !right);
  return {selected(value, value.x), false};
}

STCaretMove visual_word(STTextLayout *layout, STCaretMove current, bool right) {
  const STWord *active = nullptr;
  STCaretMove last_word = current;
  while (true) {
    int line = layout->line_at((size_t)current.byte, current.affinity == 0);
    if (line < 0)
      return {0, 0, 1, 0};
    auto step = horizontal(layout, current, line, right);
    if (step.move.status != 1)
      return step.move;
    if (step.move.byte == current.byte &&
        step.move.affinity == current.affinity)
      return active ? last_word : current;
    if (step.crossed_glyph) {
      // The arriving affinity identifies the glyph just crossed, even at a
      // coincident boundary between runs with opposite directions.
      const STWord *word =
          layout->data->word_at(step.move.byte, step.move.affinity);
      if (!word)
        return step.move;
      if (active && word != active)
        return last_word;
      if (!word->whitespace) {
        active = word;
        last_word = step.move;
      }
    }
    current = step.move;
  }
}
} // namespace

extern "C" {
STCaretMove scoop_graphics_text_move_caret(void *pointer, int64_t byte,
                                           int32_t affinity, int32_t movement,
                                           bool has_x, double preferred_x) {
  try {
    auto layout = static_cast<STTextLayout *>(pointer);
    if (!layout->data->grapheme(byte))
      return {0, 0, 1, -1};
    auto current = located(layout, byte, affinity);
    if (current.status != 1)
      return current;
    int line_index = layout->line_at((size_t)byte, affinity == 0);
    if (line_index < 0)
      return {0, 0, 1, 0};
    if (movement == 4)
      return located(layout, layout->lines[(size_t)line_index].range.start, 1);
    if (movement == 5) {
      int64_t end = (int64_t)layout->line_ends[(size_t)line_index];
      int32_t edge_affinity =
          end == layout->lines[(size_t)line_index].range.start ? 1 : 0;
      return located(layout, end, edge_affinity);
    }
    if (movement == 6)
      return located(layout, 0, 1);
    if (movement == 7)
      return located(layout, (int64_t)layout->data->text.size(), 1);

    layout->find_carets();
    const auto &line = layout->carets[(size_t)line_index];
    if (line.empty())
      return {0, 0, 1, 0};
    if (movement == 10 || movement == 11) {
      const auto &value = edge(line, movement == 11);
      return selected(value, value.x);
    }
    if (movement == 8 || movement == 9)
      return visual_word(layout, current, movement == 9);
    if (movement == 2 || movement == 3) {
      double x = has_x ? preferred_x : current.preferred_x;
      int next_line = std::clamp(line_index + (movement == 2 ? -1 : 1), 0,
                                 (int)layout->lines.size() - 1);
      if (next_line == line_index)
        return {byte, x, affinity, 1};
      const auto &target = layout->carets[(size_t)next_line];
      if (target.empty())
        return {0, 0, 1, 0};
      return selected(nearest(target, x), x);
    }
    if (movement != 0 && movement != 1)
      return {0, 0, 1, -1};
    return horizontal(layout, current, line_index, movement == 1).move;
  } catch (...) {
    return {0, 0, 1, -2};
  }
}

STRangeQuery scoop_graphics_text_logical_line(void *pointer, int64_t byte,
                                              int32_t affinity) {
  try {
    auto layout = static_cast<STTextLayout *>(pointer);
    auto &data = *layout->data;
    if (!data.grapheme(byte))
      return {{}, -1};
    if (layout->line_at((size_t)byte, affinity == 0) < 0)
      return {{}, 0};
    // Hard breaks come from the same Unicode service, including CRLF as one
    // grapheme. Logical lines can extend beyond an ellipsized visual layout.
    int64_t probe = byte > 0 && affinity == 0 ? byte - 1 : byte;
    int64_t start = 0;
    for (size_t index = 1; index < data.graphemes.size(); ++index) {
      if (data.hard_break(data.graphemes[index - 1], data.graphemes[index])) {
        int64_t line_end = data.graphemes[index];
        if (probe < line_end)
          return {{start, line_end}, 1};
        start = line_end;
      }
    }
    return {{start, (int64_t)data.text.size()}, 1};
  } catch (...) {
    return {{}, -2};
  }
}
}
