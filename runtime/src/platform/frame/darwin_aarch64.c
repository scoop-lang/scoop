#include "aarch64_fixed.h"

static bool location_frame_offset(const ScoopGcPcEntry *record, uint32_t location,
                                  uint64_t *offset) {
    uint64_t base =
        !scoop_gc_root_uses_fp(location) ? 0 : record->map->stack_size - AARCH64_FRAME_RECORD_SIZE;
    uint64_t resolved;
    if (scoop_gc_root_offset(location) < 0) {
        uint64_t magnitude = (uint64_t)(-(int64_t)scoop_gc_root_offset(location));
        if (magnitude > base)
            return false;
        resolved = base - magnitude;
    } else {
        uint64_t amount = (uint64_t)scoop_gc_root_offset(location);
        if (amount > record->map->stack_size - base)
            return false;
        resolved = base + amount;
    }
    if (resolved > record->map->stack_size ||
        record->map->stack_size - resolved < scoop_gc_root_size(location)) {
        return false;
    }
    *offset = resolved;
    return true;
}

static bool aarch64_validate_record(const ScoopGcPcEntry *record, ScoopPlatformError *error) {
    if (record == NULL || error == NULL || record->map->stack_size < AARCH64_FRAME_RECORD_SIZE ||
        record->map->stack_size % 16 != 0 || record->map->stack_size > INT64_MAX) {
        if (error != NULL)
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
        return false;
    }
    for (uint32_t index = 0; index < scoop_gc_root_count(record); index++) {
        uint32_t root = scoop_gc_root(record, index);
        uint64_t offset;
        if ((root & 4) != 0) {
            *error = (ScoopPlatformError){SCOOP_PLATFORM_UNSUPPORTED_ROOT_LOCATION,
                                          record->site->safepoint_id, index};
            return false;
        }
        if (!location_frame_offset(record, root, &offset)) {
            *error = (ScoopPlatformError){SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME,
                                          record->site->safepoint_id, index};
            return false;
        }
    }
    return true;
}

static bool aarch64_frame_from_anchor(const ScoopManagedAnchor *anchor,
                                      const ScoopGcPcEntry *record, ScoopPlatformStackBounds bounds,
                                      ScoopManagedFrame *frame, ScoopPlatformError *error) {
    bool frame_end_overflows =
        anchor != NULL && anchor->frame_pointer > UINTPTR_MAX - AARCH64_FRAME_RECORD_SIZE;
    bool stack_end_overflows = anchor != NULL && record != NULL &&
                               record->map->stack_size > UINTPTR_MAX - anchor->stack_pointer;
    if (anchor == NULL || record == NULL || frame == NULL || error == NULL || frame_end_overflows ||
        stack_end_overflows || anchor->return_pc != record->return_pc ||
        !range_contains(bounds, anchor->stack_pointer, record->map->stack_size) ||
        anchor->frame_pointer < anchor->stack_pointer ||
        anchor->frame_pointer + AARCH64_FRAME_RECORD_SIZE !=
            anchor->stack_pointer + record->map->stack_size) {
        if (error != NULL) {
            error->code = SCOOP_PLATFORM_INVALID_ANCHOR;
            error->safepoint_id = record == NULL ? 0 : record->site->safepoint_id;
        }
        return false;
    }
    *frame = (ScoopManagedFrame){
        .return_pc = anchor->return_pc,
        .stack_pointer = anchor->stack_pointer,
        .frame_pointer = anchor->frame_pointer,
        .record = record,
    };
    return true;
}

static bool aarch64_resolve_root(const ScoopManagedFrame *frame, uint32_t root_index, void ***slot,
                                 ScoopPlatformError *error) {
    if (frame == NULL || frame->record == NULL || slot == NULL || error == NULL ||
        root_index >= scoop_gc_root_count(frame->record)) {
        if (error != NULL)
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
        return false;
    }
    uint32_t location = scoop_gc_root(frame->record, root_index);
    uintptr_t base = scoop_gc_root_uses_fp(location) ? frame->frame_pointer : frame->stack_pointer;
    uintptr_t address;
    uint64_t frame_offset;
    if (!location_frame_offset(frame->record, location, &frame_offset) ||
        !address_add(base, scoop_gc_root_offset(location), &address) || address % 8 != 0 ||
        address != frame->stack_pointer + frame_offset) {
        *error = (ScoopPlatformError){SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME,
                                      frame->record->site->safepoint_id, root_index};
        return false;
    }
    *slot = (void **)address;
    return true;
}

const ScoopManagedFrameOps scoop_darwin_aarch64_managed_frame_ops = {
    .validate_record = aarch64_validate_record,
    .frame_from_anchor = aarch64_frame_from_anchor,
    .resolve_root = aarch64_resolve_root,
    .next_frame = aarch64_next_frame,
};
