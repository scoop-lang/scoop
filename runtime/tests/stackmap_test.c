#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "../src/platform/platform.h"

extern const ScoopManagedFrameOps scoop_darwin_aarch64_managed_frame_ops;

typedef struct TestMap {
    ScoopCallableGcMapV1 map;
    ScoopGcSiteV1 sites[2];
    uint32_t roots[2];
} TestMap;

static TestMap make_map(void) {
    return (TestMap){
        .map = {{SCOOP_CALLABLE_GC_MAP_MAGIC_V1, SCOOP_RUNTIME_METADATA_ABI_VERSION_V1, 32},
                64,
                2,
                2},
        .sites = {{17, 0x20, 0}, {18, 0x40, 0}},
        .roots = {1, 16},
    };
}

static void pc_index(void) {
    TestMap data = make_map();
    ScoopStackMapIndex index = {.record_count = 2};
    index.records = malloc(2 * sizeof *index.records);
    assert(index.records != NULL);
    index.records[0] = (ScoopGcPcEntry){0x1040, &data.map, &data.sites[1]};
    index.records[1] = (ScoopGcPcEntry){0x1020, &data.map, &data.sites[0]};
    uintptr_t duplicate = 0;
    assert(scoop_stackmap_finish_index(&index, &duplicate));
    const ScoopGcPcEntry *a = scoop_stackmap_lookup(&index, 0x1020);
    const ScoopGcPcEntry *b = scoop_stackmap_lookup(&index, 0x1040);
    assert(a == &index.records[0] && b == &index.records[1]);
    assert(a->site == &data.sites[0] && a->map == &data.map);
    assert(scoop_gc_root_layout(a) == data.roots);
    assert(scoop_gc_root_layout(a) == scoop_gc_root_layout(b));
    assert(scoop_gc_root_count(a) == 1 && scoop_gc_root(a, 0) == 16);
    assert(scoop_stackmap_lookup(&index, 0x1021) == NULL);
    assert(scoop_stackmap_lookup(&index, 0x101f) == NULL);
    assert(scoop_stackmap_lookup(&index, 0x1041) == NULL);
    index.records[1].return_pc = 0x1020;
    assert(!scoop_stackmap_finish_index(&index, &duplicate) && duplicate == 0x1020);
    scoop_stackmap_dispose_index(&index);
    assert(index.records == NULL && index.record_count == 0);
    assert(scoop_stackmap_finish_index(&index, &duplicate));
    assert(scoop_stackmap_lookup(&index, 0x1020) == NULL);
}

static void aarch64_roots(void) {
    TestMap data = make_map();
    ScoopGcPcEntry record = {0x1020, &data.map, &data.sites[0]};
    const ScoopManagedFrameOps *ops = &scoop_darwin_aarch64_managed_frame_ops;
    ScoopPlatformError error = {0};
    assert(ops->validate_record(&record, &error));
    _Alignas(16) uintptr_t stack[16] = {0};
    uintptr_t sp = (uintptr_t)&stack[0], fp = (uintptr_t)&stack[6];
    uintptr_t boundary = (uintptr_t)&stack[12];
    stack[6] = boundary;
    stack[7] = 0x1234;
    ScoopManagedAnchor anchor = {record.return_pc, sp, fp, NULL};
    ScoopPlatformStackBounds bounds = {(const char *)sp, (const char *)(sp + sizeof stack)};
    ScoopManagedFrame frame;
    assert(ops->frame_from_anchor(&anchor, &record, bounds, &frame, &error));
    void **slot;
    assert(ops->resolve_root(&frame, 0, &slot, &error) && slot == (void **)(sp + 16));
    data.roots[1] = (uint32_t)-32 | 1;
    assert(ops->validate_record(&record, &error));
    assert(ops->resolve_root(&frame, 0, &slot, &error) && slot == (void **)(fp - 32));
    assert(scoop_gc_root_offset(data.roots[1]) == -32 && scoop_gc_root_uses_fp(data.roots[1]));
    assert(scoop_gc_root_offset(UINT32_C(0x80000001)) == INT32_MIN);
    assert(scoop_gc_root_offset(UINT32_C(0x7ffffff9)) == INT32_MAX - 7);
    data.roots[1] = 2;
    assert(ops->validate_record(&record, &error));
    assert(scoop_gc_root_is_region_table(data.roots[1]));
    assert(scoop_gc_root_size(data.roots[1]) == 16);
    assert(ops->resolve_root(&frame, 0, &slot, &error) && slot == (void **)sp);
    data.roots[1] = 4;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_UNSUPPORTED_ROOT_LOCATION);
    data.roots[1] = 64;
    assert(!ops->validate_record(&record, &error));
    assert(error.code == SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME);
    data.roots[1] = 16;
    anchor.frame_pointer -= 8;
    assert(!ops->frame_from_anchor(&anchor, &record, bounds, &frame, &error));
    anchor.frame_pointer = fp;
    assert(ops->frame_from_anchor(&anchor, &record, bounds, &frame, &error));
    uintptr_t next_pc, next_sp, next_fp;
    bool has_next = true;
    assert(ops->next_frame(&frame, boundary, bounds, &next_pc, &next_sp, &next_fp, &has_next,
                           &error));
    assert(!has_next);
}

int main(void) {
    pc_index();
    aarch64_roots();
    puts("compact GC index tests passed");
}
