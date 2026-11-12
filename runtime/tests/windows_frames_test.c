#include "platform/platform.h"
#include "platform/primitives.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

extern void *probe0(void *), *probe1(void *), *probe2(void *), *probe3(void *), *outer(void *);

#include "compact_frames.h"

static ScoopStackMapIndex stackmaps;
static const ScoopPlatformBundle *platform;
static int before, after;
static _Thread_local uintptr_t boundary;
static _Thread_local unsigned expected_depth, visits;

/* The entry object also names runtime operations outside this ABI probe. */
#define UNUSED_ENTRY(name)                                                                     \
    void name(void) { abort(); }
UNUSED_ENTRY(scoop_rt_gc_collect_impl)
UNUSED_ENTRY(scoop_rt_prepare_throwable_impl)
UNUSED_ENTRY(scoop_rt_throwable_print_stack_trace_impl)
UNUSED_ENTRY(scoop_runtime_alloc_slow_impl)
UNUSED_ENTRY(scoop_rt_init_enter_impl)
UNUSED_ENTRY(scoop_rt_init_succeed_impl)
UNUSED_ENTRY(scoop_rt_init_fail_impl)
UNUSED_ENTRY(scoop_rt_init_failure_impl)
UNUSED_ENTRY(scoop_rt_init_cycle_message_impl)
UNUSED_ENTRY(scoop_rt_enter_native_safe_impl)
UNUSED_ENTRY(scoop_rt_enter_native_borrowed_impl)
#undef UNUSED_ENTRY

static __declspec(noinline) void check_entry_unwind(uintptr_t pc, uintptr_t sp, uintptr_t fp) {
    CONTEXT context;
    RtlCaptureContext(&context);
    unsigned depth = 0;
    while (context.Rip != pc) {
        assert(++depth < 8);
        DWORD64 image_base = 0, establisher;
        PRUNTIME_FUNCTION function = RtlLookupFunctionEntry(context.Rip, &image_base, NULL);
        assert(function != NULL);
        void *handler_data;
        RtlVirtualUnwind(UNW_FLAG_NHANDLER, image_base, context.Rip, function, &context,
                         &handler_data, &establisher, NULL);
    }
    assert(context.Rsp == sp && context.Rbp == fp);
}

static void relocate(uintptr_t pc, uintptr_t sp, uintptr_t fp) {
    check_entry_unwind(pc, sp, fp);
    ScoopPlatformStackBounds bounds;
    ScoopPlatformError error = {0};
    assert(platform->thread_vm->stack_bounds(&bounds, &error));
    unsigned depth = 0;
    for (;;) {
        const ScoopGcPcEntry *record = scoop_stackmap_lookup(&stackmaps, pc);
        assert(record != NULL && scoop_gc_root_count(record) == 1);
        ScoopManagedAnchor anchor = {pc, sp, fp, NULL};
        ScoopManagedFrame frame;
        assert(platform->managed_frames->frame_from_anchor(&anchor, record, bounds, &frame,
                                                           &error));
        if (record->site->safepoint_id == 5) {
            assert(record->map->stack_size > 16384);
            assert(fp != sp + record->map->stack_size - 8);
        }
        void **slot;
        assert(platform->managed_frames->resolve_root(&frame, 0, &slot, &error));
        assert(*slot == &before);
        *slot = &after;
        ++depth;
        bool has_next;
        assert(platform->managed_frames->next_frame(&frame, boundary, bounds, &pc, &sp, &fp,
                                                    &has_next, &error));
        if (!has_next)
            break;
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

uint64_t scoop_rt_context_push_impl(uint64_t a, uint64_t b, uint64_t c, uintptr_t pc,
                                    uintptr_t sp, uintptr_t fp) {
    assert(a == 11 && b == 22 && c == 33);
    relocate(pc, sp, fp);
    return UINT64_C(0x123456789abcdef0);
}

void touch_large(unsigned char *data) {
    assert(data[0] == 42);
    data[16383] = 17;
}

static void reject_bad_roots(void) {
    const ScoopManagedFrameOps *ops = platform->managed_frames;
    struct {
        ScoopCallableGcMapV1 map;
        ScoopGcSiteV1 site;
        uint32_t roots[2];
    } data = {.map = {.stack_size = 256, .site_count = 1, .layout_word_count = 2},
              .site = {.safepoint_id = 1},
              .roots = {1, (uint32_t)-8 | 1}};
    ScoopGcPcEntry record = {.return_pc = 1, .map = &data.map, .site = &data.site};
    _Alignas(16) void *storage[34] = {0};
    ScoopPlatformStackBounds bounds = {(char *)storage, (char *)(storage + 34)};
    ScoopManagedAnchor anchor = {1, (uintptr_t)storage, (uintptr_t)(storage + 16), NULL};
    ScoopPlatformError error = {0};
    ScoopManagedFrame frame;
    assert(ops->validate_record(&record, &error));
    assert(ops->frame_from_anchor(&anchor, &record, bounds, &frame, &error));
    void **slot;
    assert(ops->resolve_root(&frame, 0, &slot, &error) && slot == storage + 15);
    for (size_t i = 0; i < 2; ++i) {
        const int32_t offsets[] = {INT32_MIN, 128};
        data.roots[1] = (uint32_t)offsets[i] | 1;
        assert(!ops->resolve_root(&frame, 0, &slot, &error));
        assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    }
    data.roots[1] = 256;
    assert(!ops->validate_record(&record, &error));
    data.roots[1] = (uint32_t)-8;
    assert(!ops->validate_record(&record, &error));
    data.roots[1] = 2;
    assert(ops->validate_record(&record, &error));
    assert(ops->resolve_root(&frame, 0, &slot, &error) && slot == storage);
    data.roots[1] = (uint32_t)-16 | 3;
    assert(ops->validate_record(&record, &error));
    assert(ops->resolve_root(&frame, 0, &slot, &error) && slot == storage + 14);
    data.roots[1] = 248 | 2;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    data.roots[1] = 120 | 3;
    assert(ops->validate_record(&record, &error));
    assert(!ops->resolve_root(&frame, 0, &slot, &error));
    assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    data.roots[1] = 4;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_UNSUPPORTED_ROOT_LOCATION);
    anchor.stack_pointer += 8;
    assert(!ops->frame_from_anchor(&anchor, &record, bounds, &frame, &error));
}

static void *run_probes(void *unused) {
    (void)unused;
    boundary = (uintptr_t)__builtin_frame_address(0);
    expected_depth = 1;
    assert(probe0(&before) == &after);
    assert(probe1(&before) == &after);
    assert(probe2(&before) == &after);
    assert(probe3(&before) == &after);
    expected_depth = 2;
    assert(outer(&before) == &after);
    assert(visits == 5);
    return NULL;
}

int wmain(void) {
    platform = scoop_platform_bundle();
    ScoopPlatformMetadataImages images;
    ScoopPlatformError error = {0};
    assert(platform->metadata_images->loaded_images(&images, &error));
    assert(images.code_range_count == 1);
    init_probe_maps(&stackmaps);
    assert(stackmaps.record_count == 5);
    for (size_t i = 0; i < stackmaps.record_count; ++i)
        assert(platform->managed_frames->validate_record(&stackmaps.records[i], &error));
    ScoopPlatformThread worker;
    assert(scoop_runtime_platform_thread_create(&worker, run_probes, NULL) == 0);
    run_probes(NULL);
    assert(scoop_runtime_platform_thread_join(worker) == 0);
    reject_bad_roots();
    scoop_stackmap_dispose_index(&stackmaps);
    platform->metadata_images->dispose_images(&images);
    puts("Win64 entry anchors, SEH frames and relocated roots passed");
    return 0;
}
