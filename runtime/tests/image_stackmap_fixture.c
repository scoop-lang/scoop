#include <string.h>

#include "image_stackmap_fixture.h"

void scoop_test_stackmap_setup(ScoopStackmapFixture *fixture) {
    scoop_test_storage_setup(&fixture->storage, false);
    StorageMetadata *data = fixture->storage.data;
    const struct {
        ScoopCallableGcMapV1 map;
        ScoopGcSiteV1 sites[3];
        uint32_t roots[4];
    } shared = {
        .map = {{SCOOP_CALLABLE_GC_MAP_MAGIC_V1, SCOOP_RUNTIME_METADATA_ABI_VERSION_V1, 32},
                64,
                3,
                4},
        .sites = {{11, 4, 0}, {12, 8, 0}, {13, 12, 3}},
        .roots = {2, 16, 16, 0},
    };
    const struct {
        ScoopCallableGcMapV1 map;
        ScoopGcSiteV1 site;
        uint32_t roots[2];
    } empty = {
        .map = {{SCOOP_CALLABLE_GC_MAP_MAGIC_V1, SCOOP_RUNTIME_METADATA_ABI_VERSION_V1, 32},
                64,
                1,
                1},
        .site = {14, 4, 0},
        .roots = {0, 0},
    };
    memcpy(data->stackmaps, &shared, sizeof shared);
    memcpy(data->stackmaps + 128, &empty, sizeof empty);
    data->callables[1].gc_map = (const ScoopCallableGcMapV1 *)data->stackmaps;
    data->callables[0].gc_map = (const ScoopCallableGcMapV1 *)(data->stackmaps + 128);
    uintptr_t low = UINTPTR_MAX, high = 0;
    for (size_t index = 0; index < 8; index++) {
        uintptr_t entry = (uintptr_t)data->callables[index].entry;
        if (entry < low)
            low = entry;
        if (entry > high)
            high = entry;
    }
    fixture->code = (ScoopPlatformCodeRange){low, high + 16};
    fixture->storage.loaded.code_ranges = &fixture->code;
    fixture->storage.loaded.code_range_count = 1;
}
