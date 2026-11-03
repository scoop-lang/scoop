#include <limits.h>

#include "platform.h"

#ifndef SCOOP_PLATFORM_METADATA_IMAGE_OPS
#error "the runtime build must select metadata image operations"
#endif
#ifndef SCOOP_PLATFORM_THREAD_VM_OPS
#error "the runtime build must select thread and VM operations"
#endif
#ifndef SCOOP_PLATFORM_MANAGED_FRAME_OPS
#error "the runtime build must select managed frame operations"
#endif
#ifndef SCOOP_TARGET_LONG_SIZE
#error "the runtime build must select the target C long size"
#endif

typedef void (*ScoopTargetFunctionPointer)(void);

_Static_assert(CHAR_BIT == 8, "Scoop requires 8-bit bytes");
_Static_assert(sizeof(int) == 4, "Scoop target C int size");
_Static_assert(UINTPTR_MAX == UINT64_MAX, "Scoop uintptr_t value width");
_Static_assert(sizeof(uintptr_t) == 8, "Scoop uintptr_t size");
_Static_assert(_Alignof(uintptr_t) == 8, "Scoop uintptr_t alignment");
_Static_assert(sizeof(void *) == 8, "Scoop data pointer size");
_Static_assert(_Alignof(void *) == 8, "Scoop data pointer alignment");
_Static_assert(sizeof(long) == SCOOP_TARGET_LONG_SIZE, "Scoop target C long size");
_Static_assert(sizeof(ScoopTargetFunctionPointer) == 8, "Scoop function pointer size");
_Static_assert(_Alignof(ScoopTargetFunctionPointer) == 8,
               "Scoop function pointer alignment");

extern const ScoopMetadataImageOps SCOOP_PLATFORM_METADATA_IMAGE_OPS;
extern const ScoopThreadVmOps SCOOP_PLATFORM_THREAD_VM_OPS;
extern const ScoopManagedFrameOps SCOOP_PLATFORM_MANAGED_FRAME_OPS;

static const ScoopPlatformBundle platform_bundle = {
    .metadata_images = &SCOOP_PLATFORM_METADATA_IMAGE_OPS,
    .thread_vm = &SCOOP_PLATFORM_THREAD_VM_OPS,
    .managed_frames = &SCOOP_PLATFORM_MANAGED_FRAME_OPS,
};

const ScoopPlatformBundle *scoop_platform_bundle(void) { return &platform_bundle; }
