#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

#include "internal.h"

static bool code_range_contains(const ScoopPlatformMetadataImages *loaded, uintptr_t entry,
                                uintptr_t pc) {
    for (size_t index = 0; index < loaded->code_range_count; index++) {
        const ScoopPlatformCodeRange *range = &loaded->code_ranges[index];
        if (entry >= range->text_start && entry < range->text_end && pc >= range->text_start &&
            pc < range->text_end) {
            return true;
        }
    }
    return false;
}

static uint64_t map_size(const ScoopMetadataCheck *check, const ScoopCallableGcMapV1 *map) {
    scoop_metadata_prefix(check, map, SCOOP_CALLABLE_GC_MAP_MAGIC_V1, sizeof *map);
    if (map->site_count == 0 || map->layout_word_count == 0) {
        scoop_metadata_fatal(check, "empty callable GC map");
    }
    uint64_t size = sizeof *map + (uint64_t)map->site_count * sizeof(ScoopGcSiteV1) +
                    (uint64_t)map->layout_word_count * sizeof(uint32_t);
    if (size > SIZE_MAX - 7) {
        scoop_metadata_fatal(check, "callable GC map size overflow");
    }
    uint64_t aligned = (size + 7) & ~UINT64_C(7);
    scoop_metadata_range(check, map, aligned, 1, 8, "complete callable GC map range");
    return size;
}

static void read_map(const ScoopMetadataCheck *check,
                     const ScoopCallableRegistrationDescriptorV1 *callable,
                     const ScoopManagedFrameOps *frames, ScoopGcPcEntry *output) {
    const ScoopCallableGcMapV1 *map = callable->gc_map;
    /* The header and complete span were checked while sizing the PC index. */
    const uint32_t *layouts = scoop_gc_map_layouts(map);
    bool *starts = scoop_metadata_allocate(map->layout_word_count, sizeof *starts);
    for (uint32_t offset = 0; offset < map->layout_word_count;) {
        uint32_t count = layouts[offset];
        if (count > map->layout_word_count - offset - 1) {
            scoop_metadata_fatal(check, "truncated GC root layout");
        }
        starts[offset] = true;
        ScoopGcSiteV1 site = {.layout_word_offset = offset};
        ScoopGcPcEntry record = {.map = map, .site = &site};
        ScoopPlatformError error = {0};
        if (!frames->validate_record(&record, &error)) {
            fprintf(stderr, "scoop runtime GC map: root %u platform error %u\n",
                    (unsigned)error.root_index, (unsigned)error.code);
            scoop_metadata_fatal(check, "invalid managed frame or root layout");
        }
        offset += count + 1;
    }
    uintptr_t entry = (uintptr_t)callable->entry;
    const ScoopGcSiteV1 *sites = scoop_gc_map_sites(map);
    for (uint32_t index = 0; index < map->site_count; index++) {
        const ScoopGcSiteV1 *site = &sites[index];
        if (site->safepoint_id == 0 ||
            (index != 0 && sites[index - 1].instruction_offset >= site->instruction_offset) ||
            site->layout_word_offset >= map->layout_word_count ||
            !starts[site->layout_word_offset]) {
            scoop_metadata_fatal(check, "GC site order, ID or root layout offset");
        }
        if (site->instruction_offset > UINTPTR_MAX - entry ||
            !code_range_contains(check->loaded, entry, entry + site->instruction_offset)) {
            scoop_metadata_fatal(check, "GC return PC outside loaded code");
        }
        output[index] = (ScoopGcPcEntry){entry + site->instruction_offset, map, site};
    }
    free(starts);
}

void scoop_image_stackmaps(ScoopImageRegistry *registry, const ScoopManagedFrameOps *frames) {
    if (registry->callable_addresses == NULL || registry->stackmaps.records != NULL ||
        frames == NULL || frames->validate_record == NULL ||
        (registry->loaded->code_range_count != 0 && registry->loaded->code_ranges == NULL)) {
        scoop_metadata_fatal(NULL, "GC map registration order or platform");
    }
    const ScoopRecordTable *callables = &registry->tables[SCOOP_RECORD_CALLABLE];
    size_t count = 0;
    for (size_t index = 0; index < callables->count; index++) {
        const ScoopCallableRegistrationDescriptorV1 *callable =
            callables->entries[index].record;
        if (callable->gc_map == NULL)
            continue;
        ScoopMetadataCheck check = scoop_record_check(registry, SCOOP_RECORD_CALLABLE, index);
        const ScoopCallableGcMapV1 *map = callable->gc_map;
        uint64_t size = map_size(&check, map);
        const uint8_t *bytes = (const uint8_t *)map;
        for (uint64_t offset = size; offset < ((size + 7) & ~UINT64_C(7)); offset++) {
            if (bytes[offset] != 0)
                scoop_metadata_fatal(&check, "GC map padding");
        }
        if (map->site_count > SIZE_MAX - count) {
            scoop_metadata_fatal(&check, "GC PC index count overflow");
        }
        count += map->site_count;
    }
    ScoopStackMapIndex result = {.records =
                                     scoop_metadata_allocate(count, sizeof *result.records),
                                 .record_count = count};
    size_t next = 0;
    for (size_t index = 0; index < callables->count; index++) {
        const ScoopCallableRegistrationDescriptorV1 *callable =
            callables->entries[index].record;
        if (callable->gc_map == NULL)
            continue;
        ScoopMetadataCheck check = scoop_record_check(registry, SCOOP_RECORD_CALLABLE, index);
        read_map(&check, callable, frames, result.records + next);
        next += callable->gc_map->site_count;
    }
    uintptr_t duplicate = 0;
    if (!scoop_stackmap_finish_index(&result, &duplicate)) {
        fprintf(stderr, "scoop runtime GC map: duplicate return PC 0x%" PRIxPTR "\n",
                duplicate);
        scoop_metadata_fatal(NULL, "duplicate GC return PC");
    }
    registry->stackmaps = result;
}
