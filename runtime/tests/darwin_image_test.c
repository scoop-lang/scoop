#define _DARWIN_C_SOURCE
#include <assert.h>
#include <dlfcn.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>

#include "../src/platform/platform.h"

extern const ScoopMetadataImageOps scoop_macho_metadata_image_ops;
static uint64_t mutable_storage;
static const uint64_t *const descriptor = &mutable_storage;
static void managed_function(void) {}
static const uintptr_t gc_words[] __attribute__((
    used, aligned(8), section("__DATA_CONST,__scoop_gc"))) = {(uintptr_t)managed_function};

void scoop_test_check_self(const void *outside_data, const void *outside_code) {
    ScoopPlatformMetadataImages images;
    ScoopPlatformError error = {0};
    assert(scoop_macho_metadata_image_ops.loaded_images(&images, &error));
    assert(images.range_count != 0 && images.code_range_count == 1);
    assert(
        scoop_image_range_contains(&images, &descriptor, sizeof descriptor, _Alignof(void *)));
    assert(scoop_image_range_contains(&images, &mutable_storage, sizeof mutable_storage,
                                      _Alignof(uint64_t)));
    assert(scoop_image_range_contains(&images, (const void *)managed_function, 1, 1));
    assert(scoop_image_range_contains(&images, gc_words, sizeof gc_words, 8));
    assert(gc_words[0] == (uintptr_t)managed_function);
    assert(images.code_ranges[0].text_start <= (uintptr_t)managed_function);
    assert((uintptr_t)managed_function < images.code_ranges[0].text_end);
    if (outside_data != NULL)
        assert(!scoop_image_range_contains(&images, outside_data, 1, 1));
    if (outside_code != NULL)
        assert(!scoop_image_range_contains(&images, outside_code, 1, 1));
    for (size_t index = 1; index < images.range_count; ++index)
        assert(images.ranges[index - 1].end <= images.ranges[index].start);
    scoop_macho_metadata_image_ops.dispose_images(&images);
    assert(images.ranges == NULL && images.code_ranges == NULL);
}

#ifndef SCOOP_TEST_DYLIB
typedef void (*CheckImage)(const void *, const void *);
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t ready = PTHREAD_COND_INITIALIZER;
static CheckImage check_library;

static void *host_thread(void *unused) {
    (void)unused;
    assert(pthread_mutex_lock(&lock) == 0);
    while (check_library == NULL)
        assert(pthread_cond_wait(&ready, &lock) == 0);
    CheckImage check = check_library;
    assert(pthread_mutex_unlock(&lock) == 0);
    check(&mutable_storage, (const void *)managed_function);
    return NULL;
}

int main(int argc, const char *const *argv) {
    assert(argc == 2);
    scoop_test_check_self(NULL, NULL);
    pthread_t thread;
    assert(pthread_create(&thread, NULL, host_thread, NULL) == 0);
    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL)
        fprintf(stderr, "%s\n", dlerror());
    assert(library != NULL);
    CheckImage check = (CheckImage)dlsym(library, "scoop_test_check_self");
    assert(check != NULL);
    assert(pthread_mutex_lock(&lock) == 0);
    check_library = check;
    assert(pthread_cond_signal(&ready) == 0);
    assert(pthread_mutex_unlock(&lock) == 0);
    check(&mutable_storage, (const void *)managed_function);
    scoop_test_check_self(NULL, (const void *)check);
    assert(pthread_join(thread, NULL) == 0);
    assert(pthread_cond_destroy(&ready) == 0);
    assert(pthread_mutex_destroy(&lock) == 0);
    /* The module remains loaded until process exit, matching the runtime contract. */
    puts("Mach-O executable and dylib self-image ranges passed");
}
#endif
