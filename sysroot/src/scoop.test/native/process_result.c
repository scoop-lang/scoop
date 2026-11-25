#ifdef _WIN32
#define _CRT_SECURE_NO_WARNINGS
#else
#define _XOPEN_SOURCE 700
#endif
#include "test_process.h"

#include <errno.h>
#include <inttypes.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#define path_length wcslen
#define path_duplicate _wcsdup
#define result_env() _wgetenv(L"SCOOP_TEST_CASE_RESULT")
#define open_result(path, mode) _wfopen(path, L##mode)
#define remove_path _wremove
#else
#include <unistd.h>
#define path_length strlen
#define path_duplicate strdup
#define result_env() getenv("SCOOP_TEST_CASE_RESULT")
#define open_result fopen
#define remove_path remove
#endif

static TestPathChar *child_result;
static int published;

TestPathChar *scoop_test_create_result(void) {
#ifdef _WIN32
    wchar_t directory[MAX_PATH + 1], path[MAX_PATH + 1];
    DWORD count = GetTempPathW(MAX_PATH + 1, directory);
    if (!count || count > MAX_PATH || !GetTempFileNameW(directory, L"sct", 0, path))
        return NULL;
    wchar_t *result = _wcsdup(path);
    if (!result)
        _wremove(path);
    return result;
#else
    const char *temp = getenv("TMPDIR");
    char *directory = realpath(temp && *temp ? temp : "/tmp", NULL);
    if (!directory)
        return NULL;
    const char suffix[] = "/scoop-test-XXXXXX";
    size_t size = strlen(directory) + sizeof(suffix);
    char *path = malloc(size);
    if (path)
        snprintf(path, size, "%s%s", directory, suffix);
    free(directory);
    if (!path)
        return NULL;
    int fd = mkstemp(path);
    if (fd < 0) {
        free(path);
        return NULL;
    }
    close(fd);
    return path;
#endif
}

static TestPathChar *pending_path(const TestPathChar *path) {
    size_t len = path_length(path);
    TestPathChar suffix[] = {'.', 't', 'm', 'p', 0};
    TestPathChar *pending = malloc(len * sizeof(*pending) + sizeof(suffix));
    if (pending) {
        memcpy(pending, path, len * sizeof(*pending));
        memcpy(pending + len, suffix, sizeof(suffix));
    }
    return pending;
}

void scoop_test_remove_result(TestPathChar *path) {
    TestPathChar *pending = pending_path(path);
    if (pending) {
        remove_path(pending);
        free(pending);
    }
    remove_path(path);
    free(path);
}

int64_t scoop_test_child_start(void) {
    const TestPathChar *path = result_env();
    if (!path || !*path || !(child_result = path_duplicate(path)))
        return -1;
    return scoop_test_monotonic_ns();
}

int32_t scoop_test_child_publish(const uint8_t *bytes, int64_t length) {
    TestPathChar *pending = pending_path(child_result);
    if (!pending)
        return -1;
    FILE *file = open_result(pending, "wb");
    if (file) {
        int written = fwrite(bytes, 1, (size_t)length, file) == (size_t)length;
        if (fclose(file) == 0 && written) {
#ifdef _WIN32
            published = MoveFileExW(pending, child_result, MOVEFILE_REPLACE_EXISTING) != 0;
#else
            published = rename(pending, child_result) == 0;
#endif
        }
    }
    free(pending);
    free(child_result);
    child_result = NULL;
    return published ? 0 : -1;
}

int32_t scoop_test_child_published(void) { return published; }

int scoop_test_read_result(const TestPathChar *path, const char *key, struct TestRecord *record) {
    FILE *file = open_result(path, "rb");
    if (!file)
        return 0;
    char header[96];
    int valid = 0;
    char *failures = NULL;
    if (!fgets(header, sizeof(header), file))
        goto done;
    size_t key_len = strlen(key);
    if (strncmp(header, key, key_len) || header[key_len] != ' ')
        goto done;
    const char *state = header + key_len + 1;
    int passed = strncmp(state, "passed ", 7) == 0;
    if (!passed && strncmp(state, "failed ", 7))
        goto done;
    const char *number = state + 7;
    char *end;
    errno = 0;
    intmax_t duration = strtoimax(number, &end, 10);
    if (*number < '0' || *number > '9' || strcmp(end, "\n") || errno || duration > INT64_MAX)
        goto done;
    long start = ftell(file);
    if (start < 0 || fseek(file, 0, SEEK_END))
        goto done;
    long size = ftell(file);
    if (size < start || size - start < 3 || fseek(file, start, SEEK_SET))
        goto done;
    size_t length = (size_t)(size - start);
    failures = malloc(length);
    if (!failures || fread(failures, 1, length, file) != length || failures[0] != '[' ||
        failures[length - 2] != ']' || failures[length - 1] != '\n' ||
        memchr(failures, '\0', length) || memchr(failures, '\n', length - 1) ||
        passed != (length == 3))
        goto done;
    failures[length - 1] = '\0';
    *record = (struct TestRecord){passed, (int64_t)duration, failures};
    valid = 1;
done:
    if (!valid)
        free(failures);
    fclose(file);
    return valid;
}
