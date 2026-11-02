#include "../../language/callback/threading.h"
#include <assert.h>
#include <fenv.h>
#include <stdint.h>
#include <string.h>
#if defined(__x86_64__)
#include <xmmintrin.h>
#endif

typedef struct Environment {
    uint64_t control, status;
} Environment;
typedef double (*Callback)(double, double, void *);

static Environment environment(void) {
    Environment result;
#if defined(__aarch64__)
    __asm__ volatile("mrs %0, fpcr" : "=r"(result.control));
    __asm__ volatile("mrs %0, fpsr" : "=r"(result.status));
#elif defined(__x86_64__)
    uint16_t control, status;
    __asm__ volatile("fnstcw %0" : "=m"(control));
    __asm__ volatile("fnstsw %0" : "=m"(status));
    result.control = (uint64_t)control << 32 | _mm_getcsr();
    result.status = status;
#else
#error Unsupported fixture architecture
#endif
    return result;
}

static void host_environment(int32_t flavor) {
    assert(fesetenv(FE_DFL_ENV) == 0);
    assert(fesetround(flavor == 0 ? FE_UPWARD : FE_TOWARDZERO) == 0);
    assert(feraiseexcept(FE_INVALID | FE_INEXACT) == 0);
#if defined(__aarch64__)
    uint64_t control = environment().control | (UINT64_C(3) << 24) | (UINT64_C(1) << 10);
    __asm__ volatile("msr fpcr, %0" : : "r"(control));
#else
    _mm_setcsr((_mm_getcsr() | UINT32_C(0x8040)) & ~UINT32_C(0x400));
#endif
}

static void expect_environment(Environment saved) {
    Environment current = environment();
    assert(current.control == saved.control && current.status == saved.status);
}

void fp_expect_managed(void) {
    assert(fegetround() == FE_TONEAREST);
#if defined(__aarch64__)
    assert(environment().control == 0);
#else
    assert((_mm_getcsr() & UINT32_C(0xffc0)) == UINT32_C(0x1f80));
    assert((environment().control >> 32 & UINT64_C(0x3f)) == UINT64_C(0x3f));
#endif
}

void fp_pollute(void) { host_environment(0); }
void (*fp_polluter(void))(void) { return fp_pollute; }

void fp_static(double (*callback)(double, double)) {
    host_environment(0);
    Environment saved = environment();
    assert(callback(1.0, 0x1p-53) == 1.0);
    expect_environment(saved);
    double subnormal = callback(0x1p-1074, 0.0);
    uint64_t bits;
    memcpy(&bits, &subnormal, sizeof bits);
    assert(bits == 1);
    expect_environment(saved);
}

double fp_call(Callback callback, void *context, int32_t flavor, int32_t fails) {
    host_environment(flavor);
    Environment saved = environment();
    double result = callback(1.0, 0x1p-53, context);
    expect_environment(saved);
    assert(result == (fails ? 0.0 : 1.0));
    return result;
}

typedef struct Invocation {
    Callback callback;
    void *context;
    int32_t fails;
} Invocation;
static M13ThreadResult M13_THREAD_CALL invoke_thread(void *argument) {
    Invocation *invocation = argument;
    fp_call(invocation->callback, invocation->context, 1, invocation->fails);
    return 0;
}

void fp_foreign(Callback callback, void *context, int32_t fails) {
    Invocation invocation = {callback, context, fails};
    M13Thread thread;
    assert(m13_thread_create(&thread, invoke_thread, &invocation) == 0);
    assert(m13_thread_join(thread) == 0);
}
