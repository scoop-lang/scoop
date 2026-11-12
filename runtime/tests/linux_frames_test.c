#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>

#include "../src/platform/platform.h"

extern const ScoopManagedFrameOps scoop_sysv_amd64_managed_frame_ops;
extern const ScoopThreadVmOps scoop_linux_thread_vm_ops;
extern void *probe0(void *), *probe1(void *), *probe2(void *), *probe3(void *);
extern void *outer(void *);

#include "compact_frames.h"

static ScoopStackMapIndex stackmaps;
static uintptr_t boundary;
static int old_value, new_value;
static unsigned expected_depth, visits;

static void relocate(uintptr_t pc, uintptr_t sp, uintptr_t fp) {
    const ScoopManagedFrameOps *ops = &scoop_sysv_amd64_managed_frame_ops;
    ScoopPlatformStackBounds bounds;
    ScoopPlatformError error = {0};
    assert(scoop_linux_thread_vm_ops.stack_bounds(&bounds, &error));
    unsigned depth = 0;
    for (;;) {
        const ScoopGcPcEntry *record = scoop_stackmap_lookup(&stackmaps, pc);
        assert(record != NULL && scoop_gc_root_count(record) == 1);
        ScoopManagedAnchor anchor = {pc, sp, fp, NULL};
        ScoopManagedFrame frame;
        assert(ops->frame_from_anchor(&anchor, record, bounds, &frame, &error));
        void **slot;
        assert(ops->resolve_root(&frame, 0, &slot, &error));
        assert(*slot == &old_value);
        *slot = &new_value;
        ++depth;
        bool has_next;
        assert(ops->next_frame(&frame, boundary, bounds, &pc, &sp, &fp, &has_next, &error));
        if (!has_next) {
            break;
        }
    }
    assert(depth == expected_depth);
    ++visits;
}

void scoop_rt_safepoint_impl(uintptr_t pc, uintptr_t sp, uintptr_t fp) { relocate(pc, sp, fp); }
void scoop_rt_context_ensure_root_impl(uint64_t a, uintptr_t pc, uintptr_t sp, uintptr_t fp) {
    assert(a == 11);
    relocate(pc, sp, fp);
}
void scoop_rt_string_concat_impl(uint64_t a, uint64_t b, uintptr_t pc, uintptr_t sp,
                                 uintptr_t fp) {
    assert(a == 11 && b == 22);
    relocate(pc, sp, fp);
}
void scoop_rt_context_push_impl(uint64_t a, uint64_t b, uint64_t c, uintptr_t pc, uintptr_t sp,
                                uintptr_t fp) {
    assert(a == 11 && b == 22 && c == 33);
    relocate(pc, sp, fp);
}

static void reject_bad_roots(void) {
    const ScoopManagedFrameOps *ops = &scoop_sysv_amd64_managed_frame_ops;
    struct {
        ScoopCallableGcMapV1 map;
        ScoopGcSiteV1 site;
        uint32_t roots[2];
    } data = {.map = {.stack_size = 24, .site_count = 1, .layout_word_count = 2},
              .site = {.safepoint_id = 1},
              .roots = {1, 8}};
    ScoopGcPcEntry record = {.map = &data.map, .site = &data.site};
    ScoopPlatformError error = {0};
    assert(ops->validate_record(&record, &error));
    for (uint32_t offset = 16; offset <= 24; offset += 8) {
        data.roots[1] = offset;
        assert(!ops->validate_record(&record, &error));
        assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    }
    data.roots[1] = (uint32_t)-8 | 1;
    assert(ops->validate_record(&record, &error));
    data.roots[1] = 2;
    assert(ops->validate_record(&record, &error));
    data.roots[1] = (uint32_t)-16 | 3;
    assert(ops->validate_record(&record, &error));
    data.roots[1] = (uint32_t)-8 | 3;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    data.roots[1] = 10;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    data.roots[1] = 4;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_UNSUPPORTED_ROOT_LOCATION);
    data.map.stack_size = 16;
    assert(!ops->validate_record(&record, &error));
}

static void *thread_bounds(void *unused) {
    (void)unused;
    ScoopPlatformStackBounds bounds;
    ScoopPlatformError error = {0};
    assert(scoop_linux_thread_vm_ops.stack_bounds(&bounds, &error));
    assert((uintptr_t)&bounds >= (uintptr_t)bounds.low);
    assert((uintptr_t)&bounds + sizeof bounds <= (uintptr_t)bounds.high);
    return NULL;
}

int main(void) {
    init_probe_maps(&stackmaps);
    assert(stackmaps.record_count == 5);
    ScoopPlatformError error = {0};
    for (size_t i = 0; i < stackmaps.record_count; ++i) {
        assert(
            scoop_sysv_amd64_managed_frame_ops.validate_record(&stackmaps.records[i], &error));
    }
    boundary = (uintptr_t)__builtin_frame_address(0);
    expected_depth = 1;
    assert(probe0(&old_value) == &new_value);
    assert(probe1(&old_value) == &new_value);
    assert(probe2(&old_value) == &new_value);
    assert(probe3(&old_value) == &new_value);
    expected_depth = 2;
    assert(outer(&old_value) == &new_value);
    assert(visits == 5);
    reject_bad_roots();
    thread_bounds(NULL);
    pthread_t thread;
    assert(pthread_create(&thread, NULL, thread_bounds, NULL) == 0);
    assert(pthread_join(thread, NULL) == 0);
    size_t page = scoop_linux_thread_vm_ops.page_size();
    void *mapping;
    assert(page != 0);
    assert(scoop_linux_thread_vm_ops.reserve_read_write(page, page, &mapping, &error));
    *(volatile char *)mapping = 42;
    assert(!scoop_linux_thread_vm_ops.protect_none((char *)mapping + 1, page, &error));
    assert(scoop_linux_thread_vm_ops.protect_none(mapping, page, &error));
    assert(scoop_linux_thread_vm_ops.release_mapping(mapping, page, &error));
    scoop_stackmap_dispose_index(&stackmaps);
    puts("amd64 anchors, relocated roots and Linux VM passed");
}
