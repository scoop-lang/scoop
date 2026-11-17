#include "../../../native/accessibility_text_document.h"
#include <memory>

namespace {
struct Failure {
  int line;
};
#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      throw Failure{__LINE__};                                                 \
  } while (false)
struct DocumentRelease {
  void operator()(SWTextDocument *value) const {
    scoop_window_text_document_close(value);
    sw_text_document_release(value);
  }
};
using Document = std::unique_ptr<SWTextDocument, DocumentRelease>;
using Range = std::unique_ptr<SWTextRange, decltype(&sw_text_range_release)>;
int64_t insert(void *, int64_t offset, int32_t affinity) {
  return offset > 5 || (offset == 5 && affinity == 1) ? offset + 3 : offset;
}
int64_t erase(void *, int64_t offset, int32_t) {
  return offset <= 3 ? offset : offset < 10 ? 3 : offset - 7;
}
int64_t insertAtThree(void *, int64_t offset, int32_t affinity) {
  return offset > 3 || (offset == 3 && affinity == 1) ? offset + 3 : offset;
}
int64_t reject(void *, int64_t offset, int32_t) {
  return offset == 2 ? -1 : offset;
}
void checkRange(const Range &range, uint64_t revision, int64_t start,
                int64_t end) {
  auto value = sw_text_range_read(range.get(), revision);
  CHECK(value.status && value.revision == revision && value.start == start &&
        value.end == end);
}
} // namespace
extern "C" int32_t scoop_ax_test_document() {
  try {
    auto created = scoop_window_text_document_create(13);
    CHECK(created.error == 0 && created.pointer);
    Document document(static_cast<SWTextDocument *>(created.pointer));
    auto revision = sw_text_document_state(document.get()).revision;
    Range span(sw_text_range_create(document.get(), revision, 2, 7, 0, 1),
               sw_text_range_release);
    Range upstream(sw_text_range_create(document.get(), revision, 5, 5, 0, 0),
                   sw_text_range_release);
    Range downstream(sw_text_range_create(document.get(), revision, 5, 5, 1, 1),
                     sw_text_range_release);
    Range copy(sw_text_range_clone(span.get(), revision),
               sw_text_range_release);
    Range deleted(sw_text_range_create(document.get(), revision, 5, 7, 0, 1),
                  sw_text_range_release);
    CHECK(span && upstream && downstream && copy);
    CHECK(!sw_text_range_create(document.get(), revision, 0, 14, 0, 0));
    CHECK(scoop_window_text_document_map(document.get(), 16, insert, nullptr) ==
          0);
    CHECK(!sw_text_range_read(span.get(), revision).status);
    ++revision;
    checkRange(span, revision, 2, 10);
    checkRange(copy, revision, 2, 10);
    checkRange(upstream, revision, 5, 5);
    checkRange(downstream, revision, 8, 8);
    CHECK(scoop_window_text_document_map(document.get(), 16, reject, nullptr) !=
          0);
    CHECK(sw_text_document_state(document.get()).revision == revision);
    checkRange(span, revision, 2, 10);
    checkRange(downstream, revision, 8, 8);
    CHECK(scoop_window_text_document_map(document.get(), 9, erase, nullptr) ==
          0);
    ++revision;
    checkRange(span, revision, 2, 3);
    checkRange(upstream, revision, 3, 3);
    checkRange(downstream, revision, 3, 3);
    CHECK(sw_text_range_set(copy.get(), revision, 1, 2, 1, 0));
    checkRange(span, revision, 2, 3);
    checkRange(copy, revision, 1, 2);
    checkRange(deleted, revision, 3, 3);
    CHECK(scoop_window_text_document_map(document.get(), 12, insertAtThree,
                                         nullptr) == 0);
    ++revision;
    checkRange(deleted, revision, 3, 3);
    checkRange(downstream, revision, 6, 6);
    CHECK(scoop_window_text_document_reset(document.get(), 9) == 0);
    ++revision;
    CHECK(!sw_text_range_read(span.get(), revision).status);
    CHECK(!sw_text_range_clone(span.get(), revision));
    Range fresh(sw_text_range_create(document.get(), revision, 0, 9, 0, 1),
                sw_text_range_release);
    CHECK(fresh);
    document.reset();
    CHECK(!sw_text_range_read(fresh.get(), revision).status);
    CHECK(!sw_text_range_set(copy.get(), revision, 0, 1, 0, 1));
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}
