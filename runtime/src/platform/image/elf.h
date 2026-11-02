#ifndef SCOOP_PLATFORM_IMAGE_ELF_H
#define SCOOP_PLATFORM_IMAGE_ELF_H

#include <elf.h>

#include "../platform.h"

#if UINTPTR_MAX != UINT64_MAX
#error "the ELF64 image parser requires 64-bit pointers"
#endif

typedef struct ScoopElfLoadedImageView {
    uintptr_t load_bias;
    const Elf64_Phdr *program_headers;
    size_t program_header_count;
} ScoopElfLoadedImageView;

static inline bool scoop_elf_loaded_segment_range(const ScoopElfLoadedImageView *view,
                                                  const Elf64_Phdr *header, uintptr_t *start,
                                                  uintptr_t *end) {
    if (view == NULL || header == NULL || start == NULL || end == NULL ||
        header->p_vaddr > UINTPTR_MAX - view->load_bias) {
        return false;
    }
    *start = view->load_bias + (uintptr_t)header->p_vaddr;
    if (header->p_memsz > UINTPTR_MAX - *start) {
        return false;
    }
    *end = *start + (uintptr_t)header->p_memsz;
    return true;
}

bool scoop_elf_metadata_images_from_view(const ScoopElfLoadedImageView *view,
                                         const uint8_t *stackmaps_start,
                                         const uint8_t *stackmaps_end,
                                         ScoopPlatformMetadataImages *images);
void scoop_elf_dispose_metadata_images(ScoopPlatformMetadataImages *images);

#endif /* SCOOP_PLATFORM_IMAGE_ELF_H */
