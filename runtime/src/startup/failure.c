#include <stdio.h>
#include <stdlib.h>

#include "../image/registry.h"
#include "../thread/internal.h"
#include "../throwable.h"
#include "../value_shape.h"
#include "internal.h"
#include <string.h>

_Noreturn void scoop_startup_fatal(const char *message) {
    fprintf(stderr, "scoop startup: %s\n", message);
    abort();
}

static ScoopThrowableReport copy_failure(const ScoopStaticStorageDescriptorV1 *failure,
                                         const ScoopInitializationUnitDescriptorV1 *unit) {
    ScoopThrowableReport report = {0};
    for (;;) {
        scoop_thread_registry_lock();
        scoop_thread_wait_for_running_world();
        const ScoopThrowable *exception = *(const ScoopThrowable **)failure->writable_base;
        if (exception == NULL ||
            (unit != NULL && __atomic_load_n(&unit->cell->state, __ATOMIC_RELAXED) != 3)) {
            scoop_startup_fatal("gateway failure has no published exception");
        }
        scoop_image_require_type(exception->header.td);
        size_t required = scoop_throwable_report_copy(exception, &report);
        scoop_thread_registry_unlock();
        if (required == 0)
            break;
        scoop_throwable_report_reserve(&report, required);
    }
    return report;
}

_Noreturn void scoop_startup_report_failure(const ScoopStaticStorageDescriptorV1 *failure,
                                            const ScoopInitializationUnitDescriptorV1 *unit) {
    ScoopThrowableReport report = copy_failure(failure, unit);
    fputs("scoop: uncaught exception: ", stderr);
    fwrite(report.bytes, 1, report.name_length, stderr);
    if (unit != NULL) {
        fputs(" during initialization of ", stderr);
        fwrite(unit->diagnostic_path.data, 1, (size_t)unit->diagnostic_path.length, stderr);
    }
    fputc('\n', stderr);
    scoop_throwable_report_render_frames(&report, stderr);
    free(report.bytes);
    scoop_rt_exit(1);
}

typedef struct NativeReport {
    uint8_t *bytes;
    size_t length;
    size_t capacity;
} NativeReport;

static void append(void *context, const void *bytes, size_t length) {
    NativeReport *report = context;
    size_t required = scoop_shape_add(report->length, length);
    if (required > report->capacity) {
        size_t capacity = report->capacity > SIZE_MAX / 2 ? required : report->capacity * 2;
        if (capacity < required)
            capacity = required;
        void *grown = realloc(report->bytes, capacity);
        if (grown == NULL)
            scoop_startup_fatal("out of memory copying initialization failure");
        report->bytes = grown;
        report->capacity = capacity;
    }
    memcpy(report->bytes + report->length, bytes, length);
    report->length = required;
}

ScoopAppError scoop_startup_copy_failure(const ScoopStaticStorageDescriptorV1 *failure,
                                         const ScoopInitializationUnitDescriptorV1 *unit) {
    ScoopThrowableReport snapshot = copy_failure(failure, unit);
    NativeReport report = {0};
    const char *prefix = "scoop: uncaught exception: ";
    append(&report, prefix, strlen(prefix));
    append(&report, snapshot.bytes, snapshot.name_length);
    if (unit != NULL) {
        const char *during = " during initialization of ";
        append(&report, during, strlen(during));
        append(&report, unit->diagnostic_path.data, (size_t)unit->diagnostic_path.length);
    }
    append(&report, "\n", 1);
    scoop_throwable_report_write_frames(&snapshot, (ScoopTraceWriter){&report, append});
    free(snapshot.bytes);
    return (ScoopAppError){report.bytes, report.length};
}
