#define _GNU_SOURCE
#include <sched.h>
#include <sys/wait.h>
#include <unistd.h>

#include "nursery_fixture.h"

enum { NODE_COUNT = 160000 };

static void run_case(const cpu_set_t *allowed, size_t count, bool override) {
    cpu_set_t selected;
    CPU_ZERO(&selected);
    size_t remaining = count;
    for (size_t cpu = 0; cpu < CPU_SETSIZE && remaining != 0; cpu++) {
        if (CPU_ISSET(cpu, allowed)) {
            CPU_SET(cpu, &selected);
            remaining--;
        }
    }
    assert(remaining == 0);
    assert(sched_setaffinity(0, sizeof selected, &selected) == 0);
    assert(scoop_platform_bundle()->thread_vm->processor_count() == count);
    if (override) {
        assert(setenv("SCOOP_GC_WORKERS", "4", 1) == 0);
    } else {
        assert(unsetenv("SCOOP_GC_WORKERS") == 0);
    }

    const ScoopTypeDescriptor *types[] = {&nursery_node_td};
    scoop_thread_runtime_init();
    scoop_test_image_init(types, 1, NULL, 0, NULL, 0);
    scoop_thread_attach_main();
    scoop_thread_enter_managed((void *)scoop_rt_thread_debug_stack_high());
    NurseryNode *head = NULL;
    void **slot = (void **)&head;
    ScoopNativeRootFrame roots;
    scoop_rt_push_native_roots(&roots, &slot, 1);
    for (size_t index = 0; index < NODE_COUNT; index++) {
        NurseryNode *node = nursery_node(index);
        node->next = head;
        head = node;
    }
    nursery_collect(false);
    assert(nursery_metrics().last_mark_workers == (override ? 4 : count));
    assert(scoop_rt_gc_stats() == NODE_COUNT);
    NurseryNode *node = head;
    for (size_t index = NODE_COUNT; index != 0; index--) {
        assert(node != NULL && node->value == index - 1);
        node = node->next;
    }
    assert(node == NULL);
    scoop_rt_pop_native_roots(&roots);
    nursery_collect(false);
    assert(scoop_rt_gc_stats() == 0);
    scoop_gc_shutdown();
    scoop_thread_leave_managed();
    scoop_thread_detach_main();
}

int main(void) {
    cpu_set_t allowed;
    assert(sched_getaffinity(0, sizeof allowed, &allowed) == 0);
    size_t available = (size_t)CPU_COUNT(&allowed);
    assert(available > 0);
    const size_t counts[] = {1, 2, 4, 1};
    for (size_t index = 0; index < sizeof counts / sizeof *counts; index++) {
        if (counts[index] > available) {
            continue;
        }
        pid_t child = fork();
        assert(child >= 0);
        if (child == 0) {
            run_case(&allowed, counts[index], index == 3);
            _exit(0);
        }
        int status;
        assert(waitpid(child, &status, 0) == child);
        assert(WIFEXITED(status) && WEXITSTATUS(status) == 0);
    }
    puts("automatic GC workers follow CPU affinity; explicit override preserves live graphs");
}
