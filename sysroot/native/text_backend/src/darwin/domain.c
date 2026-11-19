#include "scoop_text_backend.h"
#include <limits.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unicode/uchar.h>
#include <unicode/uidna.h>
#include <unicode/utf8.h>

enum {
  empty_name = -1000,
  invalid_label = -1001,
  backend_error = -1002,
  disallowed = -1003,
  too_long = -1004,
  invalid_punycode = -1005,
  invalid_idna = -1006,
  invalid_argument = -1007
};

static bool dot(UChar32 c) {
  return c == '.' || c == 0x3002 || c == 0xff0e || c == 0xff61;
}

static int32_t check_input(const uint8_t *data, int32_t size, bool label_only,
                           bool *root, int64_t *label) {
  if (!size)
    return empty_name;
  bool ascii = true;
  for (int32_t i = 0; i < size && ascii; ++i)
    ascii = data[i] < 128;
  int64_t index = 0;
  for (int32_t i = 0; i < size;) {
    int32_t start = i;
    UChar32 c;
    U8_NEXT(data, i, size, c);
    if (c < 0) {
      *label = -1;
      return invalid_argument;
    }
    if (dot(c)) {
      if (start == 0 && i == size) {
        if (label_only) {
          *label = 0;
          return invalid_label;
        }
        *root = true;
      }
      ++index;
    } else if (c < 32 || c == 127 ||
               (ascii && !(('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z') ||
                           ('0' <= c && c <= '9') || c == '-'))) {
      if (*label < 0)
        *label = index;
    }
  }
  return *label >= 0 ? disallowed : 0;
}

static int32_t check_dns(const uint8_t *s, int32_t size, bool label_only,
                         int64_t *label) {
  if (label_only && memchr(s, '.', size))
    return invalid_label;
  if (size && s[size - 1] == '.')
    --size;
  int64_t index = 0;
  int32_t start = 0;
  for (int32_t i = 0;; ++i) {
    if (i == size || s[i] == '.') {
      if (i == start || i - start > 63) {
        *label = index;
        return i == start ? invalid_label : too_long;
      }
      if (i == size)
        break;
      start = i + 1;
      ++index;
    }
  }
  return size > 253 ? too_long : 0;
}

typedef int32_t (*Convert)(const UIDNA *, const char *, int32_t, char *,
                           int32_t, UIDNAInfo *, UErrorCode *);

static int32_t convert(const UIDNA *idna, Convert operation,
                       const uint8_t *input, int32_t size, uint8_t **out,
                       int64_t *length, uint32_t *errors) {
  UErrorCode status = U_ZERO_ERROR;
  UIDNAInfo info = UIDNA_INFO_INITIALIZER;
  int32_t len =
      operation(idna, (const char *)input, size, NULL, 0, &info, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status))
    return backend_error;
  *errors = info.errors;
  if (len <= 0)
    return *errors ? invalid_idna : invalid_label;
  uint8_t *buffer = malloc((size_t)len);
  if (!buffer)
    return backend_error;
  status = U_ZERO_ERROR;
  info = (UIDNAInfo)UIDNA_INFO_INITIALIZER;
  int32_t written = operation(idna, (const char *)input, size, (char *)buffer,
                              len, &info, &status);
  if (U_FAILURE(status) || written != len) {
    free(buffer);
    return backend_error;
  }
  *errors = info.errors;
  *out = buffer;
  *length = len;
  return 0;
}

static scoop_idna_name domain(const uint8_t *input, int64_t size,
                              bool label_only, bool unicode) {
  scoop_idna_name result = {.label = -1};
  if (size < 0 || size > INT32_MAX || (!input && size)) {
    result.code = invalid_argument;
    return result;
  }
  bool root = false;
  result.code =
      check_input(input, (int32_t)size, label_only, &root, &result.label);
  if (result.code)
    return result;
  if (root) {
    result.ascii = malloc(1);
    if (unicode)
      result.unicode = malloc(1);
    if (!result.ascii || (unicode && !result.unicode)) {
      result.code = backend_error;
      goto fail;
    }
    result.ascii[0] = '.';
    result.ascii_size = 1;
    if (unicode) {
      result.unicode[0] = '.';
      result.unicode_size = 1;
    }
    return result;
  }
  UErrorCode status = U_ZERO_ERROR;
  UIDNA *idna = uidna_openUTS46(
      UIDNA_USE_STD3_RULES | UIDNA_CHECK_BIDI | UIDNA_CHECK_CONTEXTJ |
          UIDNA_NONTRANSITIONAL_TO_ASCII | UIDNA_NONTRANSITIONAL_TO_UNICODE,
      &status);
  if (U_FAILURE(status)) {
    result.code = backend_error;
    return result;
  }
  uint32_t errors = 0;
  result.code = convert(idna, uidna_nameToASCII_UTF8, input, (int32_t)size,
                        &result.ascii, &result.ascii_size, &errors);
  uint32_t structure = UIDNA_ERROR_EMPTY_LABEL | UIDNA_ERROR_LABEL_TOO_LONG |
                       UIDNA_ERROR_DOMAIN_NAME_TOO_LONG;
  if (!result.code && (errors & ~structure))
    result.code = invalid_idna;
  if (!result.code)
    result.code = check_dns(result.ascii, (int32_t)result.ascii_size,
                            label_only, &result.label);
  if (!result.code && errors)
    result.code = invalid_idna;
  if (!result.code && unicode) {
    result.code = convert(idna, uidna_nameToUnicodeUTF8, input, (int32_t)size,
                          &result.unicode, &result.unicode_size, &errors);
    if (!result.code && errors)
      result.code = invalid_idna;
  }
  uidna_close(idna);
  if (!result.code)
    return result;
fail:
  free(result.ascii);
  free(result.unicode);
  return (scoop_idna_name){.code = result.code, .label = result.label};
}

scoop_idna_name scoop_idna_parse(const uint8_t *data, int64_t size) {
  return domain(data, size, false, true);
}

scoop_idna_result scoop_idna_domain(const uint8_t *data, int64_t size,
                                    int32_t mode) {
  if (mode < 0 || mode > 3)
    return (scoop_idna_result){.code = invalid_argument, .label = -1};
  bool unicode = (mode & 1) != 0;
  scoop_idna_name name = domain(data, size, mode >= 2, unicode);
  if (unicode)
    free(name.ascii);
  return (scoop_idna_result){.data = unicode ? name.unicode : name.ascii,
                             .size =
                                 unicode ? name.unicode_size : name.ascii_size,
                             .code = name.code,
                             .label = name.label};
}

const char *scoop_idna_message(int32_t code) {
  switch (code) {
  case empty_name:
    return "empty domain name";
  case invalid_label:
    return "empty label or separator in a single label";
  case disallowed:
    return "domain contains a disallowed DNS hostname character";
  case too_long:
    return "DNS label exceeds 63 bytes or wire name exceeds 255 bytes";
  case invalid_punycode:
    return "invalid Punycode, integer overflow, or non-scalar value";
  case invalid_idna:
    return "domain fails UTS #46 validation";
  case invalid_argument:
    return "invalid text backend argument";
  default:
    return "text backend could not complete the operation";
  }
}

const char *scoop_idna_version(void) {
  return "scoop_text_backend 0.1.0; Apple ICU";
}

static char unicode_version[U_MAX_VERSION_STRING_LENGTH];
static pthread_once_t version_once = PTHREAD_ONCE_INIT;
static void read_version(void) {
  UVersionInfo v;
  u_getUnicodeVersion(v);
  snprintf(unicode_version, sizeof(unicode_version), "%u.%u.%u", v[0], v[1],
           v[2]);
}

const char *scoop_text_unicode_version(void) {
  pthread_once(&version_once, read_version);
  return unicode_version;
}
