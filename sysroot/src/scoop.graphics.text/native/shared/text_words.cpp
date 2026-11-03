#include "text.h"

void STTextData::find_words() {
  if (words_known)
    return;
  auto iterator = unicode->makeBreakIterator(
      locale.empty() ? nullptr : locale.c_str(), SkUnicode::BreakType::kWords);
  if (!iterator || !iterator->setText(text.data(), (int)text.size()))
    throw std::runtime_error("ICU word analysis failed");
  std::vector<int64_t> boundaries = {0, (int64_t)text.size()};
  for (int offset = iterator->first(); !iterator->isDone();
       offset = iterator->next()) {
    if (grapheme(offset))
      boundaries.push_back(offset);
  }
  for (const auto &object : inlines) {
    boundaries.push_back(object.start);
    boundaries.push_back(object.end);
  }
  std::sort(boundaries.begin(), boundaries.end());
  boundaries.erase(std::unique(boundaries.begin(), boundaries.end()),
                   boundaries.end());
  std::vector<STWord> result;
  for (size_t index = 1; index < boundaries.size(); ++index) {
    STRange range = {boundaries[index - 1], boundaries[index]};
    const char *cursor = text.data() + range.start;
    const char *end = text.data() + range.end;
    bool whitespace = true;
    while (cursor < end) {
      if (!unicode->isSpace(SkUTF::NextUTF8(&cursor, end))) {
        whitespace = false;
        break;
      }
    }
    result.push_back({range, whitespace});
  }
  words = std::move(result);
  words_known = true;
}

const STWord *STTextData::word_at(int64_t byte, int32_t affinity) {
  find_words();
  if (words.empty())
    return nullptr;
  auto word = std::upper_bound(words.begin(), words.end(), byte,
                               [](int64_t offset, const STWord &item) {
                                 return offset < item.range.end;
                               });
  if (word == words.end())
    word = words.end() - 1;
  else if (affinity == 0 && word != words.begin() && word->range.start == byte)
    --word;
  return &*word;
}

extern "C" {
STRangeQuery scoop_graphics_text_word_range(void *pointer, int64_t byte,
                                            int32_t affinity) {
  try {
    auto &data = *static_cast<STTextLayout *>(pointer)->data;
    if (!data.scalar(byte))
      return {{}, -1};
    const auto *word = data.word_at(byte, affinity);
    return {word ? word->range : STRange{0, 0}, 1};
  } catch (...) {
    return {{}, -2};
  }
}

STRangeQuery scoop_graphics_text_move_word(void *pointer, int64_t byte,
                                           bool forward) {
  try {
    auto &data = *static_cast<STTextLayout *>(pointer)->data;
    if (!data.scalar(byte))
      return {{}, -1};
    data.find_words();
    int64_t next = forward ? (int64_t)data.text.size() : 0;
    if (forward) {
      auto word = std::upper_bound(data.words.begin(), data.words.end(), byte,
                                   [](int64_t offset, const STWord &item) {
                                     return offset < item.range.end;
                                   });
      while (word != data.words.end() && word->whitespace)
        ++word;
      if (word != data.words.end())
        next = word->range.end;
    } else {
      auto word = std::lower_bound(data.words.begin(), data.words.end(), byte,
                                   [](const STWord &item, int64_t offset) {
                                     return item.range.start < offset;
                                   });
      while (word != data.words.begin()) {
        --word;
        if (!word->whitespace) {
          next = word->range.start;
          break;
        }
      }
    }
    return {{next, next}, 1};
  } catch (...) {
    return {{}, -2};
  }
}

STRangeQuery scoop_graphics_text_move_grapheme(void *pointer, int64_t byte,
                                               bool forward) {
  auto &data = *static_cast<STTextLayout *>(pointer)->data;
  if (!data.scalar(byte))
    return {{}, -1};
  int64_t next = byte;
  if (forward) {
    auto edge =
        std::upper_bound(data.graphemes.begin(), data.graphemes.end(), byte);
    if (edge != data.graphemes.end())
      next = *edge;
  } else {
    auto edge =
        std::lower_bound(data.graphemes.begin(), data.graphemes.end(), byte);
    if (edge != data.graphemes.begin())
      next = *(edge - 1);
  }
  return {{next, next}, 1};
}
}
