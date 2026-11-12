#include "no_core.h"
#include <assert.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <unistd.h>

#include "image_stackmap_fixture.h"

static size_t validated_frames;

static bool validate_frame(const ScoopGcPcEntry *record, ScoopPlatformError *error) {
    validated_frames++;
    return scoop_darwin_aarch64_managed_frame_ops.validate_record(record, error);
}

static ScoopImageRegistry *check(ScoopStackmapFixture *fixture) {
    ScoopImageRegistry *registry =
        scoop_image_collect(&fixture->storage.loaded, fixture->storage.data->input, 1,
                            &fixture->storage.data->root);
    scoop_image_validate_code_and_types(registry);
    ScoopManagedFrameOps frames = scoop_darwin_aarch64_managed_frame_ops;
    frames.validate_record = validate_frame;
    scoop_image_stackmaps(registry, &frames);
    return registry;
}

static void positive(void) {
    ScoopStackmapFixture fixture = {0};
    scoop_test_stackmap_setup(&fixture);
    validated_frames = 0;
    ScoopImageRegistry *registry = check(&fixture);
    assert(registry->stackmaps.record_count == 4);
    assert(validated_frames == 3);
    const ScoopCallableGcMapV1 *map = fixture.storage.data->callables[1].gc_map;
    uintptr_t pc = (uintptr_t)fixture.storage.data->callables[1].entry + 4;
    const ScoopGcPcEntry *record = scoop_stackmap_lookup(&registry->stackmaps, pc);
    const ScoopGcPcEntry *next = scoop_stackmap_lookup(&registry->stackmaps, pc + 4);
    assert(record != NULL && record->site->safepoint_id == 11);
    assert(record->map == map && record->site == scoop_gc_map_sites(map));
    assert(scoop_gc_root_count(record) == 2);
    assert(scoop_gc_root(record, 0) == 16 && scoop_gc_root(record, 1) == 16);
    assert(scoop_gc_root_layout(record) == scoop_gc_root_layout(next));
    assert(scoop_gc_root_count(scoop_stackmap_lookup(&registry->stackmaps, pc + 8)) == 0);
    assert(scoop_stackmap_lookup(&registry->stackmaps, pc + 1) == NULL);
    scoop_image_registry_dispose(registry);
    scoop_test_storage_dispose(&fixture.storage);
}

static const char *corrupt(ScoopStackmapFixture *fixture, unsigned test) {
    StorageMetadata *data = fixture->storage.data;
    ScoopCallableGcMapV1 *map = (ScoopCallableGcMapV1 *)data->stackmaps;
    ScoopGcSiteV1 *sites = (ScoopGcSiteV1 *)(map + 1);
    uint32_t *roots = (uint32_t *)(sites + map->site_count);
    switch (test) {
    case 0:
        map->prefix.magic++;
        return "prefix magic, ABI or exact size";
    case 1:
        map->prefix.abi_version--;
        return "prefix magic, ABI or exact size";
    case 2:
        map->prefix.struct_size++;
        return "prefix magic, ABI or exact size";
    case 3:
        map->site_count = 0;
        return "empty callable GC map";
    case 4:
        map->layout_word_count = 0;
        return "empty callable GC map";
    case 5:
        map->site_count = UINT32_MAX;
        return "complete callable GC map range";
    case 6:
        data->callables[1].gc_map = (const void *)(data->stackmaps + 1);
        return "prefix range";
    case 7:
        roots[0] = 4;
        return "truncated GC root layout";
    case 8:
        sites[0].layout_word_offset = 1;
        return "GC site order, ID or root layout offset";
    case 9:
        sites[1].instruction_offset = sites[0].instruction_offset;
        return "GC site order, ID or root layout offset";
    case 10:
        sites[0].safepoint_id = 0;
        return "GC site order, ID or root layout offset";
    case 11:
        roots[1] |= 4;
        return "invalid managed frame or root layout";
    case 12:
        roots[1] = 64;
        return "invalid managed frame or root layout";
    case 13:
        map->stack_size = 0;
        return "invalid managed frame or root layout";
    case 14:
        sites[2].instruction_offset = UINT32_MAX;
        return "GC return PC outside loaded code";
    case 15:
        data->stackmaps[128 + 52] = 1;
        return "GC map padding";
    case 16: {
        ScoopCallableGcMapV1 *other = (ScoopCallableGcMapV1 *)(data->stackmaps + 128);
        ScoopGcSiteV1 *site = (ScoopGcSiteV1 *)(other + 1);
        site->instruction_offset = (uint32_t)((uintptr_t)data->callables[1].entry + 4 -
                                              (uintptr_t)data->callables[0].entry);
        return "duplicate GC return PC";
    }
    case 17:
        data->callables[1].gc_map =
            (const void *)((const uint8_t *)data + fixture->storage.size);
        return "prefix range";
    case 18:
        sites[0].layout_word_offset = map->layout_word_count;
        return "GC site order, ID or root layout offset";
    default:
        abort();
    }
}

static void negative(unsigned test) {
    ScoopStackmapFixture fixture = {0};
    scoop_test_stackmap_setup(&fixture);
    const char *expected = corrupt(&fixture, test);
    int output[2];
    assert(pipe(output) == 0);
    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        close(output[0]);
        assert(dup2(output[1], STDERR_FILENO) >= 0);
        close(output[1]);
        scoop_test_disable_core_dumps();
        scoop_image_registry_dispose(check(&fixture));
        _exit(0);
    }
    close(output[1]);
    char message[2048] = {0};
    size_t used = 0;
    ssize_t amount;
    while ((amount = read(output[0], message + used, sizeof message - used - 1)) > 0)
        used += (size_t)amount;
    close(output[0]);
    int status;
    assert(waitpid(child, &status, 0) == child);
    if (!WIFSIGNALED(status) || WTERMSIG(status) != SIGABRT ||
        strstr(message, expected) == NULL) {
        fprintf(stderr, "case %u expected %s, status %d: %s", test, expected, status, message);
        abort();
    }
    scoop_test_storage_dispose(&fixture.storage);
}

int main(void) {
    positive();
    for (unsigned test = 0; test < 19; test++)
        negative(test);
    puts("image compact GC map tests passed");
}
