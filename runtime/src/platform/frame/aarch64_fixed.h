#ifndef SCOOP_AARCH64_FIXED_FRAME_H
#define SCOOP_AARCH64_FIXED_FRAME_H

#include <string.h>

#include "../platform.h"

#if UINTPTR_MAX != UINT64_MAX
#error "the AArch64 frame decoder requires 64-bit addresses"
#endif

#define AARCH64_FRAME_RECORD_SIZE UINT64_C(16)

static bool address_add(uintptr_t base, int64_t offset, uintptr_t *result) {
    if (offset >= 0) {
        if ((uint64_t)offset > UINTPTR_MAX - base) {
            return false;
        }
        *result = base + (uintptr_t)offset;
        return true;
    }
    uint64_t magnitude = (uint64_t)(-(offset + 1)) + 1;
    if (magnitude > base) {
        return false;
    }
    *result = base - (uintptr_t)magnitude;
    return true;
}

static bool range_contains(ScoopPlatformStackBounds bounds, uintptr_t address, size_t size) {
    uintptr_t low = (uintptr_t)bounds.low;
    uintptr_t high = (uintptr_t)bounds.high;
    return low < high && address >= low && address <= high && size <= high - address;
}

static bool aarch64_next_frame(const ScoopManagedFrame *frame, uintptr_t managed_boundary,
                               ScoopPlatformStackBounds bounds, uintptr_t *return_pc,
                               uintptr_t *stack_pointer, uintptr_t *frame_pointer, bool *has_next,
                               ScoopPlatformError *error) {
    if (frame == NULL || frame->record == NULL || return_pc == NULL || stack_pointer == NULL ||
        frame_pointer == NULL || has_next == NULL || error == NULL ||
        managed_boundary <= frame->frame_pointer || managed_boundary > (uintptr_t)bounds.high ||
        !range_contains(bounds, frame->frame_pointer, AARCH64_FRAME_RECORD_SIZE)) {
        if (error != NULL) {
            error->code = SCOOP_PLATFORM_INVALID_FRAME;
            error->safepoint_id =
                frame == NULL || frame->record == NULL ? 0 : frame->record->site->safepoint_id;
        }
        return false;
    }
    uintptr_t frame_record[2];
    memcpy(frame_record, (const void *)frame->frame_pointer, sizeof frame_record);
    /* LLVM stack_size includes the frame record and the outgoing argument area. */
    uintptr_t next_stack_pointer = frame->stack_pointer + frame->record->map->stack_size;
    uintptr_t next_frame_pointer = frame_record[0];
    if (next_frame_pointer >= managed_boundary) {
        *has_next = false;
        return true;
    }
    if (next_frame_pointer <= frame->frame_pointer || next_stack_pointer >= managed_boundary ||
        frame_record[1] == 0 ||
        !range_contains(bounds, next_frame_pointer, AARCH64_FRAME_RECORD_SIZE)) {
        error->code = SCOOP_PLATFORM_INVALID_FRAME;
        error->safepoint_id = frame->record->site->safepoint_id;
        return false;
    }
    *return_pc = frame_record[1];
    *stack_pointer = next_stack_pointer;
    *frame_pointer = next_frame_pointer;
    *has_next = true;
    return true;
}

#endif
