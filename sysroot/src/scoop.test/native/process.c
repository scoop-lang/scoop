// The suite owns events and capture files. A child publishes only its private
// result.
#include "test_process.h"
#include <inttypes.h>
#include <stdlib.h>
#include <string.h>

int32_t scoop_test_isolated(const char *key, const uint8_t *name, int64_t length) {
    uint64_t ordinal = 0;
    int recording = scoop_test_begin_record((const uint8_t *)key, (int64_t)strlen(key), &ordinal);
    if (recording < 0)
        return -1;
    FILE *output = NULL, *error = NULL;
    if (recording && !getenv("SCOOP_TEST_NOCAPTURE")) {
        output = scoop_test_open_stream(ordinal, "stdout");
        error = scoop_test_open_stream(ordinal, "stderr");
        if (!output || !error) {
            if (output)
                fclose(output);
            if (error)
                fclose(error);
            return -1;
        }
    }
    struct TestExit status = {0};
    struct TestRecord record = {0};
    char reason[256] = "";
    TestPathChar *path = scoop_test_create_result();
    int spawned = 1;
    if (path)
        spawned = scoop_test_spawn(key, path, output, error, &status, reason, sizeof(reason));
    else
        snprintf(reason, sizeof(reason), "cannot create child result file");
    if (output)
        fclose(output);
    if (error)
        fclose(error);
    if (spawned == 0) {
        int received = scoop_test_read_result(path, key, &record);
        if (!status.exited)
            snprintf(reason, sizeof(reason), "test child terminated by signal %d", status.signal);
        else if (status.code != 0)
            snprintf(reason, sizeof(reason), "test child exited with code %" PRId32, status.code);
        else if (!received)
            snprintf(reason, sizeof(reason), "test child exited without a complete case result");
    }
    if (path)
        scoop_test_remove_result(path);
    int result = reason[0] || !record.passed;
    if (spawned < 0) {
        fprintf(stderr, "%s\n", reason);
        result = -1;
    } else if (recording) {
        if (scoop_test_end_isolated(key, &record, spawned == 0 ? &status : NULL,
                                    reason[0] ? reason : NULL) < 0)
            result = -1;
    } else if (reason[0]) {
        fwrite(name, 1, (size_t)length, stderr);
        fprintf(stderr, ": %s\n", reason);
    }
    free(record.failures);
    return result;
}
