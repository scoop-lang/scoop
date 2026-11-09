#include "bridge.h"
#include <stdlib.h>
#include <time.h>

#ifdef _WIN32
#include <windows.h>
#endif

static ScoopSqliteTime monotonic_now(void) {
#ifdef _WIN32
    LARGE_INTEGER count, frequency;
    QueryPerformanceCounter(&count);
    QueryPerformanceFrequency(&frequency);
    return (ScoopSqliteTime){count.QuadPart / frequency.QuadPart,
        (int32_t)((count.QuadPart % frequency.QuadPart) * 1000000000LL / frequency.QuadPart)};
#else
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) abort();
    return (ScoopSqliteTime){now.tv_sec, (int32_t)now.tv_nsec};
#endif
}

static int elapsed(ScoopSqliteTime start, ScoopSqliteTime limit) {
    if (limit.seconds < 0) return 0;
    ScoopSqliteTime now = monotonic_now();
    int64_t seconds = now.seconds - start.seconds;
    int32_t nanos = now.nanos - start.nanos;
    if (nanos < 0) { --seconds; nanos += 1000000000; }
    return seconds > limit.seconds || (seconds == limit.seconds && nanos >= limit.nanos);
}

ScoopSqlite *scoop_db_sqlite_create(int64_t busy_seconds, int32_t busy_nanos) {
    ScoopSqlite *state = calloc(1, sizeof(*state));
    if (state) {
        atomic_init(&state->database, NULL);
        atomic_init(&state->stopped, 0);
        state->busy_timeout = (ScoopSqliteTime){busy_seconds, busy_nanos};
        state->timeout.seconds = -1;
    }
    return state;
}

/* The managed owner serializes begin and retirement against cancellation callbacks. */
void scoop_db_sqlite_begin(ScoopSqlite *state, int64_t seconds, int32_t nanos) {
    state->started = monotonic_now();
    state->timeout = (ScoopSqliteTime){seconds, nanos};
    atomic_store_explicit(&state->stopped, 0, memory_order_release);
}

int32_t scoop_db_sqlite_stop(ScoopSqlite *state, int32_t reason) {
    int expected = 0;
    atomic_compare_exchange_strong_explicit(&state->stopped, &expected, reason,
        memory_order_acq_rel, memory_order_acquire);
    sqlite3 *database = atomic_load_explicit(&state->database, memory_order_acquire);
    if (database) sqlite3_interrupt(database);
    return atomic_load_explicit(&state->stopped, memory_order_acquire);
}

int32_t scoop_db_sqlite_stopped(ScoopSqlite *state) {
    int reason = atomic_load_explicit(&state->stopped, memory_order_acquire);
    if (!reason && elapsed(state->started, state->timeout)) {
        int expected = 0;
        atomic_compare_exchange_strong_explicit(&state->stopped, &expected, 2,
            memory_order_acq_rel, memory_order_acquire);
        reason = atomic_load_explicit(&state->stopped, memory_order_acquire);
    }
    return reason;
}

static int progress(void *context) { return scoop_db_sqlite_stopped(context) != 0; }

static int busy(void *context, int previous) {
    ScoopSqlite *state = context;
    if (scoop_db_sqlite_stopped(state)) return 0;
    if (previous == 0) state->busy_started = monotonic_now();
    if (elapsed(state->busy_started, state->busy_timeout)) return 0;
    sqlite3_sleep(1);
    return scoop_db_sqlite_stopped(state) == 0;
}

int32_t scoop_db_sqlite_open(ScoopSqlite *state, const char *path, int32_t readonly) {
    if (!sqlite3_threadsafe() || sqlite3_libversion_number() < 3037000) return SQLITE_MISUSE;
    sqlite3 *database = NULL;
    int flags = SQLITE_OPEN_FULLMUTEX | (readonly ? SQLITE_OPEN_READONLY : SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE);
    int code = sqlite3_open_v2(path, &database, flags, NULL);
    atomic_store_explicit(&state->database, database, memory_order_release);
    if (code == SQLITE_OK) {
        sqlite3_extended_result_codes(database, 1);
        sqlite3_progress_handler(database, 1000, progress, state);
        code = sqlite3_busy_handler(database, busy, state);
    }
    return code;
}

int32_t scoop_db_sqlite_close(ScoopSqlite *state) {
    sqlite3 *database = atomic_load_explicit(&state->database, memory_order_acquire);
    int code = database ? sqlite3_close_v2(database) : SQLITE_OK;
    if (code == SQLITE_OK) atomic_store_explicit(&state->database, NULL, memory_order_release);
    return code;
}

void scoop_db_sqlite_destroy(ScoopSqlite *state) { free(state); }

const char *scoop_db_sqlite_error(ScoopSqlite *state, int32_t code) {
    sqlite3 *database = atomic_load_explicit(&state->database, memory_order_acquire);
    return database ? sqlite3_errmsg(database) : sqlite3_errstr(code);
}

int64_t scoop_db_sqlite_changes(ScoopSqlite *state) {
    return sqlite3_total_changes64(atomic_load_explicit(&state->database, memory_order_acquire));
}

int32_t scoop_db_sqlite_autocommit(ScoopSqlite *state) {
    sqlite3 *database = atomic_load_explicit(&state->database, memory_order_acquire);
    return database ? sqlite3_get_autocommit(database) : 1;
}

int32_t scoop_db_sqlite_control(ScoopSqlite *state, const char *sql) {
    return sqlite3_exec(atomic_load_explicit(&state->database, memory_order_acquire), sql, NULL, NULL, NULL);
}
