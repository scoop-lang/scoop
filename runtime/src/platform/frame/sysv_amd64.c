#include <stdbool.h>
#include <stdint.h>
#include <string.h>

#include "../platform.h"

#if UINTPTR_MAX != UINT64_MAX
#error "the amd64 frame decoder requires 64-bit addresses"
#endif

enum { FRAME_RECORD_SIZE = 16 };

static bool range_contains(ScoopPlatformStackBounds bounds, uintptr_t address, size_t size) {
    uintptr_t low = (uintptr_t)bounds.low;
    uintptr_t high = (uintptr_t)bounds.high;
    return low < high && address >= low && address <= high && size <= high - address;
}

/* LLVM's stack size includes saved RBP, but excludes the return address. */
static bool valid_stack_size(uint64_t size) {
    return size >= 8 && size <= INT64_MAX && size % 16 == 8;
}

static bool root_frame_offset(const ScoopGcPcEntry *record, uint32_t location, uint64_t *offset) {
    uint64_t fp_offset = record->map->stack_size - 8;
    uint64_t base = !scoop_gc_root_uses_fp(location) ? 0 : fp_offset;
    int64_t displacement = scoop_gc_root_offset(location);
    uint64_t resolved;
    if (displacement < 0) {
        uint64_t amount = (uint64_t)-displacement;
        if (amount > base) {
            return false;
        }
        resolved = base - amount;
    } else {
        if ((uint64_t)displacement > fp_offset - base) {
            return false;
        }
        resolved = base + (uint64_t)displacement;
    }
    if (resolved > fp_offset || fp_offset - resolved < scoop_gc_root_size(location) ||
        resolved % 8 != 0) {
        return false;
    }
    *offset = resolved;
    return true;
}

static bool amd64_validate_record(const ScoopGcPcEntry *record, ScoopPlatformError *error) {
    if (record == NULL || error == NULL || !valid_stack_size(record->map->stack_size)) {
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
        if (!root_frame_offset(record, root, &offset)) {
            *error = (ScoopPlatformError){SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME,
                                          record->site->safepoint_id, index};
            return false;
        }
    }
    return true;
}

static bool amd64_frame_from_anchor(const ScoopManagedAnchor *anchor, const ScoopGcPcEntry *record,
                                    ScoopPlatformStackBounds bounds, ScoopManagedFrame *frame,
                                    ScoopPlatformError *error) {
    if (anchor == NULL || record == NULL || frame == NULL || error == NULL ||
        !valid_stack_size(record->map->stack_size) || anchor->return_pc != record->return_pc ||
        anchor->stack_pointer % 16 != 0 ||
        !range_contains(bounds, anchor->stack_pointer, record->map->stack_size + 8) ||
        anchor->frame_pointer != anchor->stack_pointer + record->map->stack_size - 8) {
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

static bool amd64_resolve_root(const ScoopManagedFrame *frame, uint32_t root_index, void ***slot,
                               ScoopPlatformError *error) {
    if (frame == NULL || frame->record == NULL || slot == NULL || error == NULL ||
        root_index >= scoop_gc_root_count(frame->record)) {
        if (error != NULL)
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
        return false;
    }
    uint64_t offset;
    if (!root_frame_offset(frame->record, scoop_gc_root(frame->record, root_index), &offset) ||
        (frame->stack_pointer + offset) % 8 != 0) {
        *error = (ScoopPlatformError){SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME,
                                      frame->record->site->safepoint_id, root_index};
        return false;
    }
    *slot = (void **)(frame->stack_pointer + offset);
    return true;
}

static bool amd64_next_frame(const ScoopManagedFrame *frame, uintptr_t boundary,
                             ScoopPlatformStackBounds bounds, uintptr_t *return_pc,
                             uintptr_t *stack_pointer, uintptr_t *frame_pointer, bool *has_next,
                             ScoopPlatformError *error) {
    if (frame == NULL || return_pc == NULL || stack_pointer == NULL || frame_pointer == NULL ||
        has_next == NULL || error == NULL || boundary <= frame->frame_pointer ||
        boundary > (uintptr_t)bounds.high ||
        !range_contains(bounds, frame->frame_pointer, FRAME_RECORD_SIZE)) {
        if (error != NULL) {
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
        }
        return false;
    }
    uintptr_t saved[2];
    memcpy(saved, (const void *)frame->frame_pointer, sizeof saved);
    if (saved[0] >= boundary) {
        *has_next = false;
        return true;
    }
    uintptr_t next_sp = frame->frame_pointer + FRAME_RECORD_SIZE;
    if (saved[0] <= frame->frame_pointer || next_sp >= boundary || saved[1] == 0 ||
        saved[0] % 16 != 0 || !range_contains(bounds, saved[0], FRAME_RECORD_SIZE)) {
        error->code = SCOOP_PLATFORM_INVALID_FRAME;
        error->safepoint_id = frame->record == NULL ? 0 : frame->record->site->safepoint_id;
        return false;
    }
    *return_pc = saved[1];
    *stack_pointer = next_sp;
    *frame_pointer = saved[0];
    *has_next = true;
    return true;
}

const ScoopManagedFrameOps scoop_sysv_amd64_managed_frame_ops = {
    .validate_record = amd64_validate_record,
    .frame_from_anchor = amd64_frame_from_anchor,
    .resolve_root = amd64_resolve_root,
    .next_frame = amd64_next_frame,
};
