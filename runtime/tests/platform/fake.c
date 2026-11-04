#include <pthread.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "../../src/platform/platform.h"

#if defined(__linux__)
extern const ScoopThreadVmOps scoop_linux_thread_vm_ops;
#define TEST_THREAD_VM_OPS scoop_linux_thread_vm_ops
#else
extern const ScoopThreadVmOps scoop_darwin_thread_vm_ops;
#define TEST_THREAD_VM_OPS scoop_darwin_thread_vm_ops
#endif
/* The fixture uses synthetic AArch64 frames on every host. Its OS component
 * operates on real host threads and mappings. */
extern const ScoopManagedFrameOps scoop_darwin_aarch64_managed_frame_ops;

static bool fake_loaded_images(ScoopPlatformMetadataImages *images, ScoopPlatformError *error) {
    static const ScoopPlatformCodeRange code = {0x1000, 0x2000};
    if (images == NULL || error == NULL) {
        return false;
    }
    *images = (ScoopPlatformMetadataImages){.code_ranges = &code, .code_range_count = 1};
    *error = (ScoopPlatformError){0};
    return true;
}

static void fake_dispose_images(ScoopPlatformMetadataImages *images) {
    *images = (ScoopPlatformMetadataImages){0};
}

static const ScoopMetadataImageOps fake_metadata_ops = {
    .loaded_images = fake_loaded_images,
    .dispose_images = fake_dispose_images,
};

bool scoop_test_vm_fail_mappings;
bool scoop_test_vm_fail_discards;
void (*scoop_test_vm_discard_observer)(void *base, size_t size);
static ScoopThreadVmOps fake_vm_ops;
static pthread_once_t fake_vm_once = PTHREAD_ONCE_INIT;

static bool fake_reserve(size_t size, size_t alignment, void **mapping,
                         ScoopPlatformError *error) {
    if (scoop_test_vm_fail_mappings) {
        error->code = SCOOP_PLATFORM_VM_OPERATION_FAILED;
        return false;
    }
    return TEST_THREAD_VM_OPS.reserve_read_write(size, alignment, mapping, error);
}

static bool fake_discard(void *base, size_t size, ScoopPlatformError *error) {
    if (scoop_test_vm_discard_observer != NULL) {
        scoop_test_vm_discard_observer(base, size);
    }
    if (scoop_test_vm_fail_discards) {
        error->code = SCOOP_PLATFORM_VM_OPERATION_FAILED;
        return false;
    }
    return TEST_THREAD_VM_OPS.discard_pages(base, size, error);
}

static void initialize_fake_vm(void) {
    fake_vm_ops = TEST_THREAD_VM_OPS;
    fake_vm_ops.reserve_read_write = fake_reserve;
    fake_vm_ops.discard_pages = fake_discard;
}

static const ScoopPlatformBundle fake_bundle = {
    .metadata_images = &fake_metadata_ops,
    .thread_vm = &fake_vm_ops,
    .managed_frames = &scoop_darwin_aarch64_managed_frame_ops,
};

const ScoopPlatformBundle *scoop_platform_bundle(void) {
    if (pthread_once(&fake_vm_once, initialize_fake_vm) != 0) {
        abort();
    }
    return &fake_bundle;
}

const char *scoop_platform_error_message(ScoopPlatformErrorCode code) {
    switch (code) {
    case SCOOP_PLATFORM_OK:
        return "no error";
    case SCOOP_PLATFORM_METADATA_UNAVAILABLE:
        return "fake image metadata is unavailable";
    case SCOOP_PLATFORM_INVALID_STACK_BOUNDS:
        return "fake thread stack bounds are invalid";
    case SCOOP_PLATFORM_INVALID_ANCHOR:
        return "fake managed anchor is invalid";
    case SCOOP_PLATFORM_INVALID_FRAME:
        return "fake managed frame is invalid";
    case SCOOP_PLATFORM_UNSUPPORTED_ROOT_LOCATION:
        return "fake root location is unsupported";
    case SCOOP_PLATFORM_ROOT_OUTSIDE_FRAME:
        return "fake root lies outside its frame";
    case SCOOP_PLATFORM_VM_OPERATION_FAILED:
        return "fake VM operation failed";
    }
    return "unknown fake platform error";
}

ScoopPlatformStackBounds scoop_platform_stack_bounds(void) {
    ScoopPlatformStackBounds bounds;
    ScoopPlatformError error = {0};
    if (!scoop_platform_bundle()->thread_vm->stack_bounds(&bounds, &error)) {
        fprintf(stderr, "fake platform: %s\n", scoop_platform_error_message(error.code));
        abort();
    }
    return bounds;
}
