#include "scoop_rt.h"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>

typedef struct Pair { int32_t number; double fraction; } Pair;
typedef struct Big { int64_t a, b, c; int32_t *pointer; } Big;
typedef struct Floats { float a, b, c, d; } Floats;
typedef int32_t (*Unary)(int32_t);
typedef int32_t (*Callback)(int32_t, void *);
typedef struct Table {
    int32_t (*scalars)(int8_t, uint8_t, int16_t, uint16_t, int32_t, uint32_t,
                       int64_t, uint64_t, float, double, bool);
    Pair (*pair)(Pair);
    Big (*big)(Big);
    Floats (*floats)(Floats);
    uint8_t (*enumeration)(uint8_t);
    void (*store)(int32_t *, int32_t);
    Unary (*pick)(bool);
    int32_t (*observe)(int32_t, int32_t);
    int32_t (*combine)(int32_t, int32_t);
    int32_t (*reenter)(Callback, void *, int32_t);
    int32_t (*calls)(void);
    int8_t (*narrow)(int8_t);
    float (*fraction)(double);
    bool (*boolean)(bool);
} Table;

static int32_t call_count;
static int32_t scalars(int8_t a, uint8_t b, int16_t c, uint16_t d, int32_t e,
                       uint32_t f, int64_t g, uint64_t h, float i, double j, bool k) {
    return a == -128 && b == 255 && c == -32768 && d == 65535 &&
           e == INT32_MIN && f == UINT32_MAX && g == INT64_MIN &&
           h == UINT64_MAX && i == 1.25f && j == 2.5 && k;
}
static Pair pair(Pair value) { value.number += 10; value.fraction += 0.5; return value; }
static Big big(Big value) { value.a += *value.pointer; value.b += 2; value.c += 3; return value; }
static Floats floats(Floats value) { value.a += 1; value.b += 2; value.c += 3; value.d += 4; return value; }
static uint8_t enumeration(uint8_t value) { return value; }
static void store(int32_t *pointer, int32_t value) { *pointer = value; }
static int32_t unary(int32_t value) { return value + 1; }
static Unary pick(bool present) { return present ? unary : 0; }
static int32_t observe(int32_t mode, int32_t value) {
    assert(scoop_rt_thread_debug_mode() == (uint32_t)mode);
    return value;
}
static int32_t combine(int32_t a, int32_t b) { ++call_count; return a * 10 + b; }
static int32_t reenter(Callback callback, void *context, int32_t value) {
    assert(scoop_rt_thread_debug_mode() == SCOOP_THREAD_DEBUG_NATIVE_SAFE);
    int32_t result = callback(value, context);
    assert(scoop_rt_thread_debug_mode() == SCOOP_THREAD_DEBUG_NATIVE_SAFE);
    return result;
}
static int32_t calls(void) { return call_count; }
static int8_t narrow(int8_t value) { return value; }
static float fraction(double value) { return (float)(value + 0.25); }
static bool boolean(bool value) { return !value; }
Table native_pointer_table(void) {
    return (Table){scalars, pair, big, floats, enumeration, store, pick,
                   observe, combine, reenter, calls, narrow, fraction, boolean};
}
