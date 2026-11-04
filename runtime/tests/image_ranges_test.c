#include <assert.h>
#include <stdint.h>
#include <stdio.h>

#include "../src/platform/platform.h"

extern const ScoopMetadataImageOps scoop_macho_metadata_image_ops;

static uint64_t mutable_storage;
static const uint64_t *const descriptor __attribute__((used, section("__DATA_CONST,__const"))) =
    &mutable_storage;
static void managed_function(void) {}

static const uint64_t gc_words[]
    __attribute__((used, aligned(8), section("__DATA_CONST,__scoop_gc"))) = {
        (uint64_t)(uintptr_t)managed_function};

static void synthetic_ranges(void) {
    const ScoopPlatformImageRange ranges[] = {
        {0x1000, 0x2000},
        {0x2000, 0x2800},
        {0x3000, 0x4000},
        {0x4000, 0x5000},
    };
    ScoopPlatformMetadataImages images = {.ranges = ranges, .range_count = 4};
    assert(scoop_image_range_contains(&images, (void *)0x1800, 0x1000, 8));
    assert(!scoop_image_range_contains(&images, (void *)0x1800, 0x1001, 8));
    assert(!scoop_image_range_contains(&images, (void *)0x1801, 8, 8));
    assert(scoop_image_range_contains(&images, (void *)0x3000, 8, 8));
    assert(scoop_image_range_contains(&images, (void *)0x4000, 4, 4));
    assert(!scoop_image_range_contains(&images, (void *)(UINTPTR_MAX - 7), 16, 8));
    assert(!scoop_image_range_contains(&images, NULL, 0, 8));
    assert(!scoop_image_range_contains(&images, (void *)0x1000, 8, 3));
}

int main(void) {
    synthetic_ranges();
    ScoopPlatformMetadataImages images;
    ScoopPlatformError error = {0};
    assert(scoop_macho_metadata_image_ops.loaded_images(&images, &error));
    assert(images.range_count != 0 && images.code_range_count == 1);
    assert(
        scoop_image_range_contains(&images, &descriptor, sizeof descriptor, _Alignof(void *)));
    assert(scoop_image_range_contains(&images, &mutable_storage, sizeof mutable_storage,
                                      _Alignof(uint64_t)));
    assert(scoop_image_range_contains(&images, (const void *)main, 4, 4));
    assert(scoop_image_range_contains(&images, gc_words, sizeof gc_words, 8));
    assert(gc_words[0] == (uint64_t)(uintptr_t)managed_function);
    for (size_t index = 1; index < images.range_count; index++) {
        assert(images.ranges[index - 1].end <= images.ranges[index].start);
    }
    scoop_macho_metadata_image_ops.dispose_images(&images);
    assert(images.ranges == NULL && images.code_ranges == NULL);
    puts("loaded image range tests passed");
}
