/* AAPCS64 fixes the frame record contents, but not its offset from SP. */
#include "aarch64_fixed.h"

static bool linux_validate_record(const ScoopGcPcEntry *record, ScoopPlatformError *error) {
    if (record == NULL || error == NULL || record->map->stack_size < AARCH64_FRAME_RECORD_SIZE ||
        record->map->stack_size % 16 != 0 || record->map->stack_size > INT64_MAX) {
        if (error != NULL)
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
        return false;
    }
    for (uint32_t index = 0; index < scoop_gc_root_count(record); index++) {
        uint32_t root = scoop_gc_root(record, index);
        if ((root & 4) != 0) {
            *error = (ScoopPlatformError){SCOOP_PLATFORM_UNSUPPORTED_ROOT_LOCATION,
                                          record->site->safepoint_id, index};
            return false;
        }
        int64_t offset = scoop_gc_root_offset(root);
        int64_t size = (int64_t)record->map->stack_size;
        int64_t end = offset + (int64_t)scoop_gc_root_size(root);
        bool valid = offset >= 0 && end <= size;
        if (scoop_gc_root_uses_fp(root)) {
            valid = (valid && offset >= 16) || (end <= 0 && -offset <= size - 16);
        }
        if (!valid) {
            *error = (ScoopPlatformError){SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME,
                                          record->site->safepoint_id, index};
            return false;
        }
    }
    return true;
}

static bool linux_frame_from_anchor(const ScoopManagedAnchor *anchor, const ScoopGcPcEntry *record,
                                    ScoopPlatformStackBounds bounds, ScoopManagedFrame *frame,
                                    ScoopPlatformError *error) {
    if (anchor == NULL || record == NULL || frame == NULL || error == NULL ||
        anchor->return_pc != record->return_pc || anchor->stack_pointer % 16 != 0 ||
        anchor->frame_pointer % 8 != 0 || anchor->frame_pointer < anchor->stack_pointer ||
        !range_contains(bounds, anchor->stack_pointer, record->map->stack_size) ||
        record->map->stack_size < AARCH64_FRAME_RECORD_SIZE ||
        anchor->frame_pointer - anchor->stack_pointer >
            record->map->stack_size - AARCH64_FRAME_RECORD_SIZE) {
        if (error != NULL) {
            error->code = SCOOP_PLATFORM_INVALID_ANCHOR;
            error->safepoint_id = record == NULL ? 0 : record->site->safepoint_id;
        }
        return false;
    }
    *frame = (ScoopManagedFrame){anchor->return_pc, anchor->stack_pointer, anchor->frame_pointer,
                                 record};
    return true;
}

static bool linux_resolve_root(const ScoopManagedFrame *frame, uint32_t index, void ***slot,
                               ScoopPlatformError *error) {
    if (frame == NULL || frame->record == NULL || error == NULL || slot == NULL ||
        index >= scoop_gc_root_count(frame->record)) {
        if (error != NULL)
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
        return false;
    }
    uint32_t root = scoop_gc_root(frame->record, index);
    uintptr_t base = scoop_gc_root_uses_fp(root) ? frame->frame_pointer : frame->stack_pointer;
    uintptr_t end = frame->stack_pointer + frame->record->map->stack_size;
    size_t width = scoop_gc_root_size(root);
    uintptr_t address;
    if (!address_add(base, scoop_gc_root_offset(root), &address) ||
        address < frame->stack_pointer || address > end || width > end - address ||
        (address < frame->frame_pointer + AARCH64_FRAME_RECORD_SIZE &&
         address + width > frame->frame_pointer)) {
        *error = (ScoopPlatformError){SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME,
                                      frame->record->site->safepoint_id, index};
        return false;
    }
    *slot = (void **)address;
    return true;
}

const ScoopManagedFrameOps scoop_linux_aarch64_managed_frame_ops = {
    .validate_record = linux_validate_record,
    .frame_from_anchor = linux_frame_from_anchor,
    .resolve_root = linux_resolve_root,
    .next_frame = aarch64_next_frame,
};
