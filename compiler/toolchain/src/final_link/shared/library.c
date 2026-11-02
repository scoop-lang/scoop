#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unwind.h>

static _Thread_local int thread_value = 40;

int scoop_shared_helper(void) { return 42; }

#ifdef __linux__
__attribute__((section(".data.rel.ro.scoop"))) int (*scoop_shared_metadata)(void) =
    scoop_shared_helper;
#endif

static _Unwind_Reason_Code trace(struct _Unwind_Context *context, void *data) {
    (void)context;
    (*(int *)data)++;
    return _URC_NO_REASON;
}

static __attribute__((noinline)) int check(void) {
    int frames = 0;
    _Unwind_Backtrace(trace, &frames);
    if (frames < 2 || scoop_shared_helper() != 42) {
        fprintf(stderr, "callback frames: %d, helper: %d\n", frames, scoop_shared_helper());
        return -1;
    }
    int *value = malloc(sizeof(*value));
    if (!value)
        return -2;
    *value = ++thread_value;
    int result = *value;
    free(value);
#ifdef SCOOP_TEST_CPP
    extern int scoop_cpp_probe(int);
    if (scoop_cpp_probe(0) != 10 || scoop_cpp_probe(1) != 10)
        return -3;
#endif
    return result;
}

int scoop_app_init(const void *host, void *output, void *error) {
    (void)host;
    (void)error;
    struct {
        int (*check)(void);
    } api = {check};
    memcpy(output, &api, sizeof(api));
    return 0;
}
