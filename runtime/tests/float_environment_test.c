#include <assert.h>
#include <fenv.h>
#include <pthread.h>
#include <stdio.h>

#include "../src/managed_entries.h"
#include "../src/startup/internal.h"
#include "../src/thread.h"
#include "platform/image_fixture.h"

static void host_environment(int rounding) {
    assert(fesetenv(FE_DFL_ENV) == 0);
    assert(fesetround(rounding) == 0);
    assert(feraiseexcept(FE_INVALID | FE_INEXACT) == 0);
}

static void expect_host(int rounding) {
    assert(fegetround() == rounding);
    assert(fetestexcept(FE_ALL_EXCEPT) == (FE_INVALID | FE_INEXACT));
}

static void expect_managed(void) {
    assert(fegetround() == FE_TONEAREST);
    assert(fetestexcept(FE_ALL_EXCEPT) == 0);
}

static uint32_t gateway_status;

static uint32_t gateway(void) {
    expect_managed();
    _Alignas(16) uintptr_t frames[2][4] = {{0}};
    frames[1][2] = (uintptr_t)scoop_thread_current_required()->managed_stack_boundary;
    frames[0][2] = (uintptr_t)frames[1];
    scoop_rt_safepoint_impl(0x1010, (uintptr_t)frames[1], (uintptr_t)&frames[1][2]);

    ScoopCallerRootFrame roots[2];
    ScoopThreadTransition transitions[2];
    ScoopCallbackThreadEntry callback = {0};
    scoop_rt_push_caller_roots(&roots[0], NULL, 0);
    scoop_rt_enter_native_safe_impl(&transitions[0], (uintptr_t)frames[1], 0x1010,
                                    (uintptr_t)frames[1], (uintptr_t)&frames[1][2]);
    host_environment(FE_UPWARD);
    scoop_thread_enter_callback(&callback, frames[1]);
    expect_managed();
    scoop_rt_push_caller_roots(&roots[1], NULL, 0);
    scoop_rt_enter_native_borrowed_impl(&transitions[1], (uintptr_t)frames[0], 0x1010,
                                        (uintptr_t)frames[0], (uintptr_t)&frames[0][2]);
    host_environment(FE_TOWARDZERO);
    scoop_rt_leave_native_borrowed(&transitions[1]);
    expect_managed();
    scoop_rt_pop_caller_roots(&roots[1]);
    scoop_thread_leave_callback(&callback);
    expect_host(FE_UPWARD);
    scoop_rt_leave_native_safe(&transitions[0]);
    expect_managed();
    scoop_rt_pop_caller_roots(&roots[0]);
    return gateway_status;
}

static void *foreign_thread(void *unused) {
    (void)unused;
    host_environment(FE_TOWARDZERO);
    assert(scoop_rt_attach_foreign_thread());
    expect_host(FE_TOWARDZERO);
    assert(scoop_startup_call_gateway(gateway) == gateway_status);
    expect_host(FE_TOWARDZERO);
    scoop_rt_detach_foreign_thread();
    expect_host(FE_TOWARDZERO);
    return NULL;
}

int main(void) {
    host_environment(FE_DOWNWARD);
    scoop_thread_runtime_init();
    scoop_test_image_init(NULL, 0, NULL, 0, NULL, 0);
    scoop_thread_attach_main();
    expect_host(FE_DOWNWARD);
    for (gateway_status = 0; gateway_status != 2; ++gateway_status) {
        assert(scoop_startup_call_gateway(gateway) == gateway_status);
        expect_host(FE_DOWNWARD);
        pthread_t thread;
        assert(pthread_create(&thread, NULL, foreign_thread, NULL) == 0);
        assert(pthread_join(thread, NULL) == 0);
    }
    scoop_thread_prepare_shutdown();
    scoop_thread_detach_main();
    scoop_thread_runtime_finish_shutdown();
    expect_host(FE_DOWNWARD);
    puts("floating environment gateway and transition tests passed");
}
