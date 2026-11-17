#include "accessibility_text_document.h"
#include <limits.h>
#include <stdlib.h>
#ifdef _WIN32
#include <windows.h>
typedef SRWLOCK DocumentLock;
static void lock_init(DocumentLock *lock) { InitializeSRWLock(lock); }
static void lock_acquire(DocumentLock *lock) { AcquireSRWLockExclusive(lock); }
static void lock_release(DocumentLock *lock) { ReleaseSRWLockExclusive(lock); }
static void lock_destroy(DocumentLock *lock) { (void)lock; }
#else
#include <pthread.h>
typedef pthread_mutex_t DocumentLock;
static void lock_init(DocumentLock *lock) { pthread_mutex_init(lock, NULL); }
static void lock_acquire(DocumentLock *lock) { pthread_mutex_lock(lock); }
static void lock_release(DocumentLock *lock) { pthread_mutex_unlock(lock); }
static void lock_destroy(DocumentLock *lock) { pthread_mutex_destroy(lock); }
#endif

struct SWTextDocument {
  DocumentLock lock;
  uint64_t references, revision;
  int64_t length;
  int32_t alive;
  SWTextRange *ranges;
};
struct SWTextRange {
  SWTextDocument *document;
  SWTextRange *previous, *next;
  int64_t start, end, staged_start, staged_end;
  int32_t start_affinity, end_affinity, alive;
};

SWTextDocumentCreate scoop_window_text_document_create(int64_t length) {
  if (length < 0)
    return (SWTextDocumentCreate){NULL, -4};
  SWTextDocument *document = calloc(1, sizeof(*document));
  if (!document)
    return (SWTextDocumentCreate){NULL, 1};
  lock_init(&document->lock);
  document->references = 1;
  document->revision = 1;
  document->length = length;
  document->alive = 1;
  return (SWTextDocumentCreate){document, 0};
}
void sw_text_document_retain(SWTextDocument *document) {
  if (!document)
    return;
  lock_acquire(&document->lock);
  ++document->references;
  lock_release(&document->lock);
}
void sw_text_document_release(SWTextDocument *document) {
  if (!document)
    return;
  lock_acquire(&document->lock);
  int last = --document->references == 0;
  lock_release(&document->lock);
  if (last) {
    lock_destroy(&document->lock);
    free(document);
  }
}
SWTextDocumentState sw_text_document_state(SWTextDocument *document) {
  if (!document)
    return (SWTextDocumentState){0};
  lock_acquire(&document->lock);
  SWTextDocumentState state = {document->revision, document->length,
                               document->alive};
  lock_release(&document->lock);
  return state;
}
static void retire_ranges(SWTextDocument *document) {
  for (SWTextRange *range = document->ranges; range; range = range->next)
    range->alive = 0;
}
void scoop_window_text_document_close(SWTextDocument *document) {
  if (!document)
    return;
  lock_acquire(&document->lock);
  document->alive = 0;
  retire_ranges(document);
  lock_release(&document->lock);
}
int64_t scoop_window_text_document_reset(SWTextDocument *document,
                                         int64_t length) {
  if (!document || length < 0)
    return -4;
  lock_acquire(&document->lock);
  if (!document->alive || document->revision == UINT64_MAX) {
    lock_release(&document->lock);
    return -4;
  }
  retire_ranges(document);
  document->length = length;
  ++document->revision;
  lock_release(&document->lock);
  return 0;
}
int64_t scoop_window_text_document_map(SWTextDocument *document, int64_t length,
                                       SWTextOffsetMapper mapper,
                                       void *context) {
  if (!document || length < 0 || !mapper)
    return -4;
  lock_acquire(&document->lock);
  if (!document->alive || document->revision == UINT64_MAX) {
    lock_release(&document->lock);
    return -4;
  }
  for (SWTextRange *range = document->ranges; range; range = range->next) {
    if (!range->alive)
      continue;
    range->staged_start = mapper(context, range->start, range->start_affinity);
    range->staged_end = mapper(context, range->end, range->end_affinity);
    if (range->staged_start < 0 || range->staged_end < 0 ||
        range->staged_start > length || range->staged_end > length) {
      lock_release(&document->lock);
      return -4;
    }
    if (range->staged_start > range->staged_end)
      range->staged_end = range->staged_start;
  }
  for (SWTextRange *range = document->ranges; range; range = range->next)
    if (range->alive) {
      range->start = range->staged_start;
      range->end = range->staged_end;
      if (range->start == range->end)
        range->end_affinity = range->start_affinity;
    }
  document->length = length;
  ++document->revision;
  lock_release(&document->lock);
  return 0;
}
static int valid_range(SWTextDocument *document, uint64_t revision,
                       int64_t start, int64_t end, int32_t first,
                       int32_t last) {
  return document->alive && document->revision == revision && start >= 0 &&
         end >= start && end <= document->length &&
         (first == 0 || first == 1) && (last == 0 || last == 1);
}
SWTextRange *sw_text_range_create(SWTextDocument *document, uint64_t revision,
                                  int64_t start, int64_t end, int32_t first,
                                  int32_t last) {
  if (!document)
    return NULL;
  SWTextRange *range = calloc(1, sizeof(*range));
  if (!range)
    return NULL;
  lock_acquire(&document->lock);
  if (!valid_range(document, revision, start, end, first, last)) {
    lock_release(&document->lock);
    free(range);
    return NULL;
  }
  range->document = document;
  range->alive = 1;
  range->start = start;
  range->end = end;
  range->start_affinity = first;
  range->end_affinity = start == end ? first : last;
  range->next = document->ranges;
  if (range->next)
    range->next->previous = range;
  document->ranges = range;
  ++document->references;
  lock_release(&document->lock);
  return range;
}
SWTextRangeState sw_text_range_read(SWTextRange *range, uint64_t revision) {
  if (!range)
    return (SWTextRangeState){0};
  SWTextDocument *document = range->document;
  lock_acquire(&document->lock);
  SWTextRangeState state = {range->start,
                            range->end,
                            document->revision,
                            range->start_affinity,
                            range->end_affinity,
                            range->alive && document->alive &&
                                document->revision == revision};
  lock_release(&document->lock);
  return state;
}
SWTextRange *sw_text_range_clone(SWTextRange *range, uint64_t revision) {
  SWTextRangeState state = sw_text_range_read(range, revision);
  return state.status
             ? sw_text_range_create(range->document, revision, state.start,
                                    state.end, state.start_affinity,
                                    state.end_affinity)
             : NULL;
}
int32_t sw_text_range_set(SWTextRange *range, uint64_t revision, int64_t start,
                          int64_t end, int32_t first, int32_t last) {
  if (!range)
    return 0;
  SWTextDocument *document = range->document;
  lock_acquire(&document->lock);
  int valid =
      range->alive && valid_range(document, revision, start, end, first, last);
  if (valid) {
    range->start = start;
    range->end = end;
    range->start_affinity = first;
    range->end_affinity = start == end ? first : last;
  }
  lock_release(&document->lock);
  return valid;
}
void sw_text_range_release(SWTextRange *range) {
  if (!range)
    return;
  SWTextDocument *document = range->document;
  lock_acquire(&document->lock);
  if (range->previous)
    range->previous->next = range->next;
  else
    document->ranges = range->next;
  if (range->next)
    range->next->previous = range->previous;
  lock_release(&document->lock);
  free(range);
  sw_text_document_release(document);
}
