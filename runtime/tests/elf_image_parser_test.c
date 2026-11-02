#include <assert.h>
#include <stdio.h>

#include "../src/platform/image/elf.h"

static _Alignas(16) uint8_t image_storage[256];

static ScoopElfLoadedImageView valid_view(Elf64_Phdr headers[2]) {
    headers[0] = (Elf64_Phdr){
        .p_type = PT_LOAD,
        .p_flags = PF_R | PF_W,
        .p_vaddr = 128,
        .p_memsz = 128,
    };
    headers[1] = (Elf64_Phdr){
        .p_type = PT_LOAD,
        .p_flags = PF_R | PF_X,
        .p_vaddr = 0,
        .p_memsz = 128,
    };
    return (ScoopElfLoadedImageView){
        .load_bias = (uintptr_t)image_storage,
        .program_headers = headers,
        .program_header_count = 2,
        .stackmaps_start = image_storage + 128,
        .stackmaps_end = image_storage + 144,
    };
}

static void parses_explicit_image_view(void) {
    Elf64_Phdr headers[2];
    ScoopElfLoadedImageView view = valid_view(headers);
    uintptr_t start, end;
    assert(scoop_elf_loaded_segment_range(&view, &headers[1], &start, &end));
    assert(start == (uintptr_t)image_storage && end == (uintptr_t)image_storage + 128);

    ScoopPlatformMetadataImages images;
    assert(scoop_elf_metadata_images_from_view(&view, &images));
    assert(images.range_count == 2 && images.count == 1);
    assert(images.ranges[0].start == (uintptr_t)image_storage);
    assert(images.ranges[0].end == (uintptr_t)image_storage + 128);
    assert(images.ranges[1].start == (uintptr_t)image_storage + 128);
    assert(images.ranges[1].end == (uintptr_t)image_storage + 256);
    assert(images.images[0].text_start == (uintptr_t)image_storage);
    assert(images.images[0].text_end == (uintptr_t)image_storage + 128);
    assert(images.images[0].section == image_storage + 128);
    assert(images.images[0].section_size == 16);
    scoop_elf_dispose_metadata_images(&images);
    assert(images.ranges == NULL && images.images == NULL);
}

static void rejects_invalid_views(void) {
    Elf64_Phdr headers[2];
    ScoopElfLoadedImageView view = valid_view(headers);
    ScoopPlatformMetadataImages images = {0};

    headers[0].p_vaddr = 64;
    assert(!scoop_elf_metadata_images_from_view(&view, &images));
    assert(images.ranges == NULL && images.images == NULL);

    view = valid_view(headers);
    headers[1].p_flags = PF_R;
    assert(!scoop_elf_metadata_images_from_view(&view, &images));

    view = valid_view(headers);
    view.stackmaps_start = image_storage + 248;
    view.stackmaps_end = (const uint8_t *)((uintptr_t)image_storage + 264);
    assert(!scoop_elf_metadata_images_from_view(&view, &images));

    view = valid_view(headers);
    view.load_bias = UINTPTR_MAX - 3;
    headers[1].p_vaddr = 4;
    assert(!scoop_elf_metadata_images_from_view(&view, &images));

    view = (ScoopElfLoadedImageView){.program_header_count = 1};
    assert(!scoop_elf_metadata_images_from_view(&view, &images));
}

int main(void) {
    parses_explicit_image_view();
    rejects_invalid_views();
    puts("ELF image parser view tests passed");
}
