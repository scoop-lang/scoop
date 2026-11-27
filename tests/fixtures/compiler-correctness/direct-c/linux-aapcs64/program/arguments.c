#include <assert.h>
#include <stdint.h>

typedef struct { double first, second; } Doubles;
typedef struct __attribute__((aligned(16))) { uint8_t bytes[15]; } Fifteen;

Doubles round_trip_hfa(double a, double b, double c, double d, double e, double f,
                       double g, double h, int64_t i, int64_t j, int64_t k, int64_t l,
                       int64_t m, int64_t n, int64_t o, int64_t p, int64_t prefix,
                       Doubles value, int64_t suffix) {
    assert(a + b + c + d + e + f + g + h == 36.0);
    assert(i + j + k + l + m + n + o + p == 36);
    assert(prefix == 91 && value.first == 9.5 && value.second == 10.5 && suffix == 117);
    return (Doubles){value.second, value.first + 0.5};
}

typedef Doubles (*HfaCall)(double, double, double, double, double, double, double, double,
                           int64_t, int64_t, int64_t, int64_t, int64_t, int64_t, int64_t, int64_t,
                           int64_t, Doubles, int64_t);

HfaCall get_hfa_call(void) { return round_trip_hfa; }

Fifteen round_trip_aligned(int64_t prefix, Fifteen value, int64_t suffix) {
    assert(prefix == 31 && suffix == 43);
    for (unsigned i = 0; i < sizeof(value.bytes); ++i) {
        assert(value.bytes[i] == i + 1);
        value.bytes[i] += 16;
    }
    return value;
}
