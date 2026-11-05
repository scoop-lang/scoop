#define _GNU_SOURCE
#include <link.h>

#include "../image/elf.h"

#if !defined(__linux__) || UINTPTR_MAX != UINT64_MAX
#error "the Linux ELF loaded-image provider requires 64-bit Linux"
#endif

_Static_assert(sizeof(ElfW(Phdr)) == sizeof(Elf64_Phdr), "Linux ELF program header size");
_Static_assert(_Alignof(ElfW(Phdr)) == _Alignof(Elf64_Phdr),
               "Linux ELF program header alignment");

typedef struct LinuxElfImageQuery {
    ScoopPlatformMetadataImages *images;
    bool loaded;
} LinuxElfImageQuery;

static bool linux_elf_loaded_images(ScoopPlatformMetadataImages *images,
                                    ScoopPlatformError *error);

static ScoopElfLoadedImageView loaded_image_view(const struct dl_phdr_info *info) {
    return (ScoopElfLoadedImageView){
        .load_bias = (uintptr_t)info->dlpi_addr,
        .program_headers = (const Elf64_Phdr *)info->dlpi_phdr,
        .program_header_count = info->dlpi_phnum,
    };
}

static int collect_runtime_image(struct dl_phdr_info *info, size_t size, void *data) {
    (void)size;
    LinuxElfImageQuery *query = data;
    ScoopElfLoadedImageView view = loaded_image_view(info);
    uintptr_t own_code = (uintptr_t)linux_elf_loaded_images;
    for (size_t index = 0; index < view.program_header_count; ++index) {
        const Elf64_Phdr *header = &view.program_headers[index];
        uintptr_t start, end;
        if (header->p_type == PT_LOAD &&
            scoop_elf_loaded_segment_range(&view, header, &start, &end) && own_code >= start &&
            own_code < end) {
            query->loaded = scoop_elf_metadata_images_from_view(&view, query->images);
            return 1;
        }
    }
    return 0;
}

static bool linux_elf_loaded_images(ScoopPlatformMetadataImages *images,
                                    ScoopPlatformError *error) {
    if (images == NULL || error == NULL) {
        return false;
    }
    *images = (ScoopPlatformMetadataImages){0};
    LinuxElfImageQuery query = {.images = images};
    dl_iterate_phdr(collect_runtime_image, &query);
    if (!query.loaded) {
        scoop_elf_dispose_metadata_images(images);
        error->code = SCOOP_PLATFORM_METADATA_UNAVAILABLE;
        return false;
    }
    *error = (ScoopPlatformError){0};
    return true;
}

const ScoopMetadataImageOps scoop_linux_elf_metadata_image_ops = {
    .loaded_images = linux_elf_loaded_images,
    .dispose_images = scoop_elf_dispose_metadata_images,
};
