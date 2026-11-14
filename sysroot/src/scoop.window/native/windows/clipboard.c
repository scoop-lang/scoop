#include "window.h"
#include <string.h>

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWClipboardText;

typedef struct SWClipboardEntry {
  UINT format;
  HGLOBAL memory;
  struct SWClipboardEntry *next;
} SWClipboardEntry;

typedef struct {
  SWClipboardEntry *first, *last;
} SWClipboardWriter;

static UINT clipboard_format(const uint8_t *bytes, int64_t length) {
  if (length == 10 && memcmp(bytes, "text/plain", 10) == 0)
    return CF_UNICODETEXT;
  WCHAR *name = sw_wide(bytes, length);
  if (!name)
    return 0;
  UINT format = RegisterClipboardFormatW(name);
  free(name);
  return format;
}

static SWClipboardText clipboard_read_utf8(HANDLE handle) {
  SWClipboardText result = {0};
  const uint8_t *text = GlobalLock(handle);
  if (!text)
    return (SWClipboardText){.error = GetLastError()};
  SIZE_T size = GlobalSize(handle);
  SIZE_T length = 0;
  while (length < size && text[length])
    ++length;
  // A registered text format may include a terminator and zero allocation
  // padding. Keep any actual embedded NUL for the shared text contract to
  // reject.
  for (SIZE_T index = length + 1; index < size; ++index) {
    if (text[index]) {
      length = index + 1;
      break;
    }
  }
  if (length > INT64_MAX) {
    result.error = ERROR_INVALID_DATA;
  } else {
    result.bytes = malloc(length ? length : 1);
    if (result.bytes) {
      if (length)
        memcpy(result.bytes, text, length);
      result.length = (int64_t)length;
    } else {
      result.error = ERROR_NOT_ENOUGH_MEMORY;
    }
  }
  GlobalUnlock(handle);
  return result;
}

SWClipboardText scoop_window_clipboard_read(const uint8_t *type_bytes,
                                            int64_t type_length) {
  UINT format = clipboard_format(type_bytes, type_length);
  if (!format)
    return (SWClipboardText){.error = GetLastError()};
  if (!IsClipboardFormatAvailable(format))
    return (SWClipboardText){0};
  if (!OpenClipboard(NULL))
    return (SWClipboardText){.error = GetLastError()};
  SWClipboardText result = {0};
  HANDLE handle = GetClipboardData(format);
  if (!handle) {
    result.error = GetLastError();
    CloseClipboard();
    return result;
  }
  if (format != CF_UNICODETEXT) {
    result = clipboard_read_utf8(handle);
    CloseClipboard();
    return result;
  }
  const WCHAR *text = GlobalLock(handle);
  if (!text) {
    result.error = GetLastError();
    CloseClipboard();
    return result;
  }
  SIZE_T size = GlobalSize(handle) / sizeof(WCHAR);
  SIZE_T length = 0;
  while (length < size && text[length])
    ++length;
  SIZE_T content_length = length;
  for (SIZE_T index = length + 1; index < size; ++index) {
    if (text[index]) {
      content_length = index + 1;
      break;
    }
  }
  if (content_length > INT_MAX)
    result.error = ERROR_INVALID_DATA;
  else {
    result.bytes = sw_utf8(text, (int)content_length, &result.length);
    if (!result.bytes)
      result.error = GetLastError();
  }
  GlobalUnlock(handle);
  CloseClipboard();
  return result;
}

void scoop_window_clipboard_release(uint8_t *bytes) { free(bytes); }

static HGLOBAL clipboard_plain_text(const uint8_t *bytes, int64_t length,
                                    DWORD *error) {
  WCHAR *text = sw_wide(bytes, length);
  if (!text) {
    *error = GetLastError();
    return NULL;
  }
  SIZE_T count = 0;
  for (WCHAR *cursor = text; *cursor; ++cursor)
    count += *cursor == L'\n' ? 2 : 1;
  if (count > (SIZE_MAX / sizeof(WCHAR)) - 1) {
    free(text);
    *error = ERROR_ARITHMETIC_OVERFLOW;
    return NULL;
  }
  HGLOBAL memory =
      GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, (count + 1) * sizeof(WCHAR));
  if (!memory) {
    free(text);
    *error = GetLastError();
    return NULL;
  }
  WCHAR *output = GlobalLock(memory);
  if (!output) {
    *error = GetLastError();
    GlobalFree(memory);
    free(text);
    return NULL;
  }
  SIZE_T index = 0;
  for (WCHAR *cursor = text; *cursor; ++cursor) {
    if (*cursor == L'\n')
      output[index++] = L'\r';
    output[index++] = *cursor;
  }
  output[index] = 0;
  GlobalUnlock(memory);
  free(text);
  return memory;
}

static HGLOBAL clipboard_utf8_text(const uint8_t *bytes, int64_t length,
                                   DWORD *error) {
  if (length < 0 || (uint64_t)length > SIZE_MAX - 1) {
    *error = ERROR_ARITHMETIC_OVERFLOW;
    return NULL;
  }
  HGLOBAL memory =
      GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, (SIZE_T)length + 1);
  if (!memory) {
    *error = GetLastError();
    return NULL;
  }
  uint8_t *output = GlobalLock(memory);
  if (!output) {
    *error = GetLastError();
    GlobalFree(memory);
    return NULL;
  }
  if (length)
    memcpy(output, bytes, (SIZE_T)length);
  output[length] = 0;
  GlobalUnlock(memory);
  return memory;
}

SWResult scoop_window_clipboard_create(void) {
  SWClipboardWriter *writer = calloc(1, sizeof(*writer));
  return (SWResult){writer, writer ? 0 : ERROR_NOT_ENOUGH_MEMORY};
}

int64_t scoop_window_clipboard_add(void *value, const uint8_t *type_bytes,
                                   int64_t type_length, const uint8_t *bytes,
                                   int64_t length) {
  SWClipboardWriter *writer = value;
  UINT format = clipboard_format(type_bytes, type_length);
  if (!format)
    return GetLastError();
  SWClipboardEntry *entry = calloc(1, sizeof(*entry));
  if (!entry)
    return ERROR_NOT_ENOUGH_MEMORY;
  DWORD error = 0;
  entry->format = format;
  entry->memory = format == CF_UNICODETEXT
                      ? clipboard_plain_text(bytes, length, &error)
                      : clipboard_utf8_text(bytes, length, &error);
  if (!entry->memory) {
    free(entry);
    return error ? error : ERROR_NOT_ENOUGH_MEMORY;
  }
  if (writer->last)
    writer->last->next = entry;
  else
    writer->first = entry;
  writer->last = entry;
  return 0;
}

int64_t scoop_window_clipboard_write(void *value, SWApplication *application) {
  SWClipboardWriter *writer = value;
  if (!OpenClipboard(application->hwnd)) {
    return GetLastError();
  }
  DWORD error = 0;
  if (!EmptyClipboard())
    error = GetLastError() ? GetLastError() : ERROR_WRITE_FAULT;
  for (SWClipboardEntry *entry = writer->first; !error && entry;
       entry = entry->next) {
    if (SetClipboardData(entry->format, entry->memory))
      entry->memory = NULL;
    else
      error = GetLastError() ? GetLastError() : ERROR_WRITE_FAULT;
  }
  CloseClipboard();
  return error;
}

void scoop_window_clipboard_destroy(void *value) {
  SWClipboardWriter *writer = value;
  SWClipboardEntry *entry = writer->first;
  while (entry) {
    SWClipboardEntry *next = entry->next;
    if (entry->memory)
      GlobalFree(entry->memory);
    free(entry);
    entry = next;
  }
  free(writer);
}
