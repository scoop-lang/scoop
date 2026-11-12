#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../src/gc_binding/internal.h"
#include "../src/thread.h"
#include "platform/image_fixture.h"

typedef struct VisitState {
    void **expected_slot;
    void *replacement;
    uint64_t visits;
} VisitState;

static void replace_root(void **slot, void *raw_state) {
    VisitState *state = raw_state;
    assert(slot == state->expected_slot);
    *slot = state->replacement;
    state->visits++;
}

typedef struct RegionState {
    const ScoopCallerRootEntry *entries;
    void *replacement;
    uint64_t visits;
} RegionState;

static void unexpected_slot(void **slot, void *context) {
    (void)slot;
    (void)context;
    assert(0 && "a region table must not be visited as a single reference slot");
}

static void replace_region(void *base, const uint64_t *scan, void *raw_state) {
    RegionState *state = raw_state;
    assert(state->visits < 2);
    assert(base == state->entries[state->visits].base);
    assert(scan == state->entries[state->visits].scan);
    *(void **)base = state->replacement;
    state->visits++;
}

int main(void) {
    ScoopStackMapIndex index = scoop_test_stackmaps();
    scoop_gc_stackmaps_init(&index);

    _Alignas(16) uintptr_t stack[16] = {0};
    void *original = (void *)(uintptr_t)0x1230;
    void *replacement = (void *)(uintptr_t)0x4560;
    memcpy(&stack[0], &original, sizeof original);

    uintptr_t boundary = (uintptr_t)&stack[8];
    uintptr_t frame_record[2] = {boundary, 0};
    memcpy(&stack[2], frame_record, sizeof frame_record);
    ScoopManagedAnchor anchor = {
        .return_pc = 0x1010,
        .stack_pointer = (uintptr_t)&stack[0],
        .frame_pointer = (uintptr_t)&stack[2],
        .previous = NULL,
    };
    ScoopThreadState thread = {
        .stack_low = (const char *)&stack[0],
        .stack_high = (const char *)&stack[16],
        .managed_stack_boundary = (const char *)boundary,
        .managed_anchor = &anchor,
    };
    VisitState state = {
        .expected_slot = (void **)&stack[0],
        .replacement = replacement,
        .visits = 0,
    };
    ScoopGcRootVisitor visitor = {
        .visit_slot = replace_root,
        .visit_region = NULL,
        .context = &state,
    };
    scoop_gc_visit_managed_stack(&thread, visitor);

    void *updated = NULL;
    memcpy(&updated, &stack[0], sizeof updated);
    assert(state.visits == 1);
    assert(updated == replacement);

    memcpy(&stack[0], &original, sizeof original);
    state.visits = 0;
    scoop_gc_visit_managed_segment(&thread, anchor.return_pc, anchor.stack_pointer,
                                   anchor.frame_pointer, boundary, visitor);
    memcpy(&updated, &stack[0], sizeof updated);
    assert(state.visits == 1);
    assert(updated == replacement);

    static const struct {
        ScoopCallableGcMapV1 map;
        ScoopGcSiteV1 site;
        uint32_t roots[2];
    } regions = {
        .map = {{SCOOP_CALLABLE_GC_MAP_MAGIC_V1, SCOOP_RUNTIME_METADATA_ABI_VERSION_V1, 32},
                32,
                1,
                2},
        .site = {1, 0x10, 0},
        .roots = {1, 2},
    };
    index.records[0] = (ScoopGcPcEntry){0x1010, &regions.map, &regions.site};
    ScoopPlatformError error = {0};
    assert(scoop_platform_bundle()->managed_frames->validate_record(&index.records[0], &error));
    uint64_t scans[2] = {0, 0};
    void *values[2] = {original, original};
    ScoopCallerRootEntry entries[2] = {{&values[0], &scans[0]}, {&values[1], &scans[1]}};
    ScoopStackRootTable table = {entries, 2};
    memcpy(&stack[0], &table, sizeof table);
    RegionState region_state = {entries, replacement, 0};
    visitor = (ScoopGcRootVisitor){
        .visit_slot = unexpected_slot, .visit_region = replace_region, .context = &region_state};
    scoop_gc_visit_managed_stack(&thread, visitor);
    assert(region_state.visits == 2);
    assert(values[0] == replacement && values[1] == replacement);

    values[0] = original;
    values[1] = original;
    region_state.visits = 0;
    scoop_gc_visit_managed_segment(&thread, anchor.return_pc, anchor.stack_pointer,
                                   anchor.frame_pointer, boundary, visitor);
    assert(region_state.visits == 2);
    assert(values[0] == replacement && values[1] == replacement);

    scoop_stackmap_dispose_index(&index);
    puts("exact stack root walk passed");
    return 0;
}
