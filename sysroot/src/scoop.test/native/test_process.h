#ifndef SCOOP_TEST_PROCESS_H
#define SCOOP_TEST_PROCESS_H

#include "test_io.h"
#ifdef _WIN32
#include <wchar.h>
typedef wchar_t TestPathChar;
#else
typedef char TestPathChar;
#endif

struct TestExit {
    int exited;
    int32_t code;
    int signal;
};

struct TestRecord {
    int passed;
    int64_t duration_ns;
    char *failures;
};

TestPathChar *scoop_test_create_result(void);
void scoop_test_remove_result(TestPathChar *path);
int scoop_test_read_result(const TestPathChar *path, const char *key, struct TestRecord *record);
// 0 means reaped, 1 means the process did not start, -1 means the wait failed.
int scoop_test_spawn(const char *key, const TestPathChar *result, FILE *output, FILE *error,
                     struct TestExit *status, char *reason, size_t capacity);
int scoop_test_begin_record(const uint8_t *key, int64_t length, uint64_t *ordinal);
int scoop_test_end_isolated(const char *key, const struct TestRecord *record,
                            const struct TestExit *status, const char *interruption);

#endif
