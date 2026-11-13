#include "bridge.h"
#include <limits.h>
#include <stddef.h>
#include <string.h>

ScoopSqlitePrepared scoop_db_sqlite_prepare(ScoopSqlite *state, const char *sql) {
    sqlite3 *database = atomic_load_explicit(&state->database, memory_order_acquire);
    sqlite3_stmt *statement = NULL;
    const char *cursor = sql;
    while (*cursor) {
        if (scoop_db_sqlite_stopped(state)) {
            sqlite3_finalize(statement);
            return (ScoopSqlitePrepared){NULL, SQLITE_INTERRUPT};
        }
        sqlite3_stmt *next = NULL;
        const char *tail = NULL;
        int code = sqlite3_prepare_v3(database, cursor, -1, 0, &next, &tail);
        if (code != SQLITE_OK) {
            sqlite3_finalize(next);
            sqlite3_finalize(statement);
            return (ScoopSqlitePrepared){NULL, code};
        }
        if (next) {
            if (statement) {
                sqlite3_finalize(next);
                sqlite3_finalize(statement);
                return (ScoopSqlitePrepared){NULL, -2};
            }
            statement = next;
        }
        if (!tail || tail == cursor) break;
        cursor = tail;
    }
    return (ScoopSqlitePrepared){statement, statement ? SQLITE_OK : -1};
}

int32_t scoop_db_sqlite_bind_bytes(sqlite3_stmt *statement, int32_t slot,
    const int8_t *data, int64_t length, int32_t text) {
    if (text) return sqlite3_bind_text64(statement, slot, (const char *)data,
        (sqlite3_uint64)length, SQLITE_TRANSIENT, SQLITE_UTF8);
    if (length == 0) return sqlite3_bind_zeroblob(statement, slot, 0);
    return sqlite3_bind_blob64(statement, slot, data, (sqlite3_uint64)length, SQLITE_TRANSIENT);
}

int32_t scoop_db_sqlite_reset(sqlite3_stmt *statement) {
    int code = sqlite3_reset(statement);
    int cleared = sqlite3_clear_bindings(statement);
    return cleared != SQLITE_OK ? cleared : code;
}
