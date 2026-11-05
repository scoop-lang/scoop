/* Fixture initialization must also run when testing an optimized runtime. */
#undef NDEBUG
#include <assert.h>
#include <stdlib.h>

#include "image_fixture.h"

ScoopStackMapIndex scoop_test_stackmaps(void) {
    static const struct {
        ScoopCallableGcMapV1 map;
        ScoopGcSiteV1 site;
        uint32_t roots[2];
    } data = {
        .map = {{SCOOP_CALLABLE_GC_MAP_MAGIC_V1, SCOOP_RUNTIME_METADATA_ABI_VERSION_V1, 32},
                32,
                1,
                2},
        .site = {1, 0x10, 0},
        .roots = {1, 0},
    };
    ScoopStackMapIndex index = {.record_count = 1};
    index.records = malloc(sizeof *index.records);
    assert(index.records != NULL);
    index.records[0] = (ScoopGcPcEntry){0x1010, &data.map, &data.site};
    ScoopPlatformError error = {0};
    assert(scoop_platform_bundle()->managed_frames->validate_record(&index.records[0], &error));
    return index;
}
