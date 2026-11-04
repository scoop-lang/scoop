#ifndef SCOOP_TEST_IMAGE_STACKMAP_FIXTURE_H
#define SCOOP_TEST_IMAGE_STACKMAP_FIXTURE_H

#include "image_storage_fixture.h"

typedef struct ScoopStackmapFixture {
    ScoopStorageFixture storage;
    ScoopPlatformCodeRange code;
} ScoopStackmapFixture;

extern const ScoopManagedFrameOps scoop_darwin_aarch64_managed_frame_ops;
void scoop_test_stackmap_setup(ScoopStackmapFixture *fixture);

#endif
