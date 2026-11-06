#include "window.h"

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWClipboardText;

SWClipboardText scoop_window_clipboard_read(void) {
  if (!IsClipboardFormatAvailable(CF_UNICODETEXT))
    return (SWClipboardText){0};
  if (!OpenClipboard(NULL))
    return (SWClipboardText){.error = GetLastError()};
  SWClipboardText result = {0};
  HANDLE handle = GetClipboardData(CF_UNICODETEXT);
  if (!handle) {
    result.error = GetLastError();
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

int64_t scoop_window_clipboard_write(const uint8_t *bytes, int64_t length) {
  WCHAR *text = sw_wide(bytes, length);
  if (!text)
    return GetLastError();
  SIZE_T count = 0;
  for (WCHAR *cursor = text; *cursor; ++cursor)
    count += *cursor == L'\n' ? 2 : 1;
  if (count > (SIZE_MAX / sizeof(WCHAR)) - 1) {
    free(text);
    return ERROR_ARITHMETIC_OVERFLOW;
  }
  HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE, (count + 1) * sizeof(WCHAR));
  if (!memory) {
    free(text);
    return GetLastError();
  }
  WCHAR *output = GlobalLock(memory);
  if (!output) {
    DWORD error = GetLastError();
    GlobalFree(memory);
    free(text);
    return error;
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
  if (!OpenClipboard(NULL)) {
    DWORD error = GetLastError();
    GlobalFree(memory);
    return error;
  }
  DWORD error = 0;
  if (!EmptyClipboard())
    error = GetLastError();
  if (!error && !SetClipboardData(CF_UNICODETEXT, memory))
    error = GetLastError();
  CloseClipboard();
  if (error)
    GlobalFree(memory);
  return error;
}
