#ifndef SCOOP_WINDOW_ACCESSIBILITY_TEXT_DOCUMENT_H
#define SCOOP_WINDOW_ACCESSIBILITY_TEXT_DOCUMENT_H

#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct SWTextDocument SWTextDocument;
typedef struct SWTextRange SWTextRange;
typedef struct {
  void *pointer;
  int64_t error;
} SWTextDocumentCreate;
typedef struct {
  uint64_t revision;
  int64_t length;
  int32_t alive;
} SWTextDocumentState;
typedef struct {
  int64_t start, end;
  uint64_t revision;
  int32_t start_affinity, end_affinity, status;
} SWTextRangeState;
typedef int64_t (*SWTextOffsetMapper)(void *, int64_t, int32_t);

SWTextDocumentCreate scoop_window_text_document_create(int64_t length);
void sw_text_document_retain(SWTextDocument *document);
void sw_text_document_release(SWTextDocument *document);
SWTextDocumentState sw_text_document_state(SWTextDocument *document);
void scoop_window_text_document_close(SWTextDocument *document);
int64_t scoop_window_text_document_reset(SWTextDocument *document,
                                         int64_t length);
/* The synchronous mapper must not reenter the document or perform UI
 * operations. */
int64_t scoop_window_text_document_map(SWTextDocument *document, int64_t length,
                                       SWTextOffsetMapper mapper,
                                       void *context);
SWTextRange *sw_text_range_create(SWTextDocument *document, uint64_t revision,
                                  int64_t start, int64_t end,
                                  int32_t start_affinity, int32_t end_affinity);
SWTextRange *sw_text_range_clone(SWTextRange *range, uint64_t revision);
SWTextRangeState sw_text_range_read(SWTextRange *range, uint64_t revision);
int32_t sw_text_range_set(SWTextRange *range, uint64_t revision, int64_t start,
                          int64_t end, int32_t start_affinity,
                          int32_t end_affinity);
void sw_text_range_release(SWTextRange *range);

#ifdef __cplusplus
}
#endif
#endif
