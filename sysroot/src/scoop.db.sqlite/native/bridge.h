#ifndef SCOOP_DB_SQLITE_BRIDGE_H
#define SCOOP_DB_SQLITE_BRIDGE_H

#include <sqlite3.h>
#include <stdatomic.h>
#include <stdint.h>

typedef struct {
    int64_t seconds;
    int32_t nanos;
} ScoopSqliteTime;

typedef struct {
    _Atomic(sqlite3 *) database;
    atomic_int stopped;
    ScoopSqliteTime started;
    ScoopSqliteTime timeout;
    ScoopSqliteTime busy_started;
    ScoopSqliteTime busy_timeout;
} ScoopSqlite;

typedef struct {
    sqlite3_stmt *statement;
    int32_t code;
} ScoopSqlitePrepared;

int32_t scoop_db_sqlite_stop(ScoopSqlite *state, int32_t reason);
int32_t scoop_db_sqlite_stopped(ScoopSqlite *state);

#endif
