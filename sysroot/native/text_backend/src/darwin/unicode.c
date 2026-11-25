#include "scoop_text_backend.h"
#include <CoreFoundation/CoreFoundation.h>
#include <limits.h>
#include <stdlib.h>
#include <string.h>
#include <unicode/uchar.h>
#include <unicode/ustring.h>
#include <unicode/utf16.h>
#include <unicode/utf8.h>

static scoop_text_result failure(void) {
  return (scoop_text_result){.code = 9};
}

/* The caller handles empty spans. Each UTF-8 byte accounts for at most one
 * UTF-16 code unit, including surrogate pairs for supplementary scalars. */
static UChar *to_utf16(const uint8_t *data, int64_t size, int32_t *length) {
  if (!data || size <= 0 || size > INT32_MAX)
    return NULL;
  UChar *units = malloc((size_t)size * sizeof(*units));
  if (!units)
    return NULL;
  UErrorCode status = U_ZERO_ERROR;
  u_strFromUTF8(units, (int32_t)size, length, (const char *)data, (int32_t)size,
                &status);
  if (U_FAILURE(status)) {
    free(units);
    return NULL;
  }
  return units;
}

static CFStringRef literal_string(const uint8_t *data, int64_t size) {
  int32_t length;
  UChar *units = to_utf16(data, size, &length);
  if (!units)
    return NULL;
  // The character constructor preserves an initial U+FEFF as ordinary text.
  CFStringRef result =
      CFStringCreateWithCharacters(NULL, (const UniChar *)units, length);
  free(units);
  return result;
}

scoop_text_result scoop_charset_normalize_nfc(const uint8_t *data,
                                              int64_t size) {
  if (!size)
    return (scoop_text_result){0};
  CFStringRef literal = literal_string(data, size);
  if (!literal)
    return failure();
  CFMutableStringRef text = CFStringCreateMutableCopy(NULL, 0, literal);
  CFRelease(literal);
  if (!text)
    return failure();
  CFStringNormalize(text, kCFStringNormalizationFormC);
  CFRange range = CFRangeMake(0, CFStringGetLength(text));
  CFIndex bytes;
  CFIndex units = CFStringGetBytes(text, range, kCFStringEncodingUTF8, 0, false,
                                   NULL, 0, &bytes);
  scoop_text_result result = {0};
  if (units != range.length) {
    result = failure();
  } else if (bytes) {
    result.data = malloc((size_t)bytes);
    CFIndex written = 0;
    if (!result.data ||
        CFStringGetBytes(text, range, kCFStringEncodingUTF8, 0, false,
                         result.data, bytes, &written) != units ||
        written != bytes) {
      free(result.data);
      result = failure();
    } else {
      result.size = bytes;
    }
  }
  CFRelease(text);
  return result;
}

bool scoop_charset_alphanumeric(uint32_t scalar) {
  return scalar <= 0x10ffff &&
         u_hasBinaryProperty((UChar32)scalar, UCHAR_POSIX_ALNUM);
}

static int32_t map_case(int32_t mode, UChar *out, int32_t capacity,
                        const UChar *text, int32_t length, UErrorCode *status) {
  switch (mode) {
  case 0:
    return u_strToLower(out, capacity, text, length, "", status);
  case 1:
    return u_strToUpper(out, capacity, text, length, "", status);
  default:
    return u_strFoldCase(out, capacity, text, length, U_FOLD_CASE_DEFAULT,
                         status);
  }
}

static scoop_text_result to_utf8(const UChar *text, int32_t length) {
  if (!length)
    return (scoop_text_result){0};
  UErrorCode status = U_ZERO_ERROR;
  int32_t size = 0;
  u_strToUTF8(NULL, 0, &size, text, length, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR || !size)
    return failure();
  uint8_t *bytes = malloc((size_t)size);
  if (!bytes)
    return failure();
  status = U_ZERO_ERROR;
  int32_t written = 0;
  u_strToUTF8((char *)bytes, size, &written, text, length, &status);
  if (U_FAILURE(status) || written != size) {
    free(bytes);
    return failure();
  }
  return (scoop_text_result){.data = bytes, .size = size};
}

scoop_text_result scoop_charset_case(const uint8_t *data, int64_t size,
                                     int32_t mode) {
  if (mode < 0 || mode > 2)
    return failure();
  if (!size)
    return (scoop_text_result){0};
  int32_t length;
  UChar *text = to_utf16(data, size, &length);
  if (!text)
    return failure();
  UErrorCode status = U_ZERO_ERROR;
  int32_t needed = map_case(mode, NULL, 0, text, length, &status);
  if (needed == 0 && U_SUCCESS(status)) {
    free(text);
    return (scoop_text_result){0};
  }
  if (status != U_BUFFER_OVERFLOW_ERROR || needed <= 0) {
    free(text);
    return failure();
  }
  UChar *out = malloc((size_t)needed * sizeof(*out));
  if (!out) {
    free(text);
    return failure();
  }
  status = U_ZERO_ERROR;
  int32_t written = map_case(mode, out, needed, text, length, &status);
  free(text);
  scoop_text_result result = U_SUCCESS(status) && needed == written
                                 ? to_utf8(out, written)
                                 : failure();
  free(out);
  return result;
}
