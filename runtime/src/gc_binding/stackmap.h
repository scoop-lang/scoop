#ifndef SCOOP_GC_STACKMAP_H
#define SCOOP_GC_STACKMAP_H

#include "../../include/scoop_runtime_metadata_v1.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/* The roots and frame size stay in the immutable loaded image. */
typedef struct ScoopGcPcEntry {
    uintptr_t return_pc;
    const ScoopCallableGcMapV1 *map;
    const ScoopGcSiteV1 *site;
} ScoopGcPcEntry;

_Static_assert(sizeof(ScoopGcPcEntry) == 24, "compact GC PC index entry");

typedef struct ScoopStackMapIndex {
    ScoopGcPcEntry *records;
    size_t record_count;
} ScoopStackMapIndex;

static inline const ScoopGcSiteV1 *scoop_gc_map_sites(const ScoopCallableGcMapV1 *map) {
    return (const ScoopGcSiteV1 *)(map + 1);
}
static inline const uint32_t *scoop_gc_map_layouts(const ScoopCallableGcMapV1 *map) {
    return (const uint32_t *)(scoop_gc_map_sites(map) + map->site_count);
}
static inline const uint32_t *scoop_gc_root_layout(const ScoopGcPcEntry *record) {
    return scoop_gc_map_layouts(record->map) + record->site->layout_word_offset;
}
static inline uint32_t scoop_gc_root_count(const ScoopGcPcEntry *record) {
    return scoop_gc_root_layout(record)[0];
}
static inline uint32_t scoop_gc_root(const ScoopGcPcEntry *record, uint32_t index) {
    return scoop_gc_root_layout(record)[1 + index];
}
static inline int32_t scoop_gc_root_offset(uint32_t encoded) {
    uint32_t bits = encoded & UINT32_C(0xfffffff8);
    int32_t offset;
    memcpy(&offset, &bits, sizeof offset);
    return offset;
}
static inline bool scoop_gc_root_uses_fp(uint32_t encoded) { return (encoded & 1) != 0; }
static inline bool scoop_gc_root_is_region_table(uint32_t encoded) { return (encoded & 2) != 0; }
static inline size_t scoop_gc_root_size(uint32_t encoded) {
    return scoop_gc_root_is_region_table(encoded) ? 16 : 8;
}

bool scoop_stackmap_finish_index(ScoopStackMapIndex *index, uintptr_t *duplicate_pc);
void scoop_stackmap_dispose_index(ScoopStackMapIndex *index);
const ScoopGcPcEntry *scoop_stackmap_lookup(const ScoopStackMapIndex *index, uintptr_t return_pc);

#endif
