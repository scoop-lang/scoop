#include <stdlib.h>

#include "elf.h"

static bool append_range(ScoopPlatformImageRange **ranges, size_t *count, uintptr_t start,
                         uintptr_t end) {
    if (*count == SIZE_MAX / sizeof **ranges) {
        return false;
    }
    ScoopPlatformImageRange *next = realloc(*ranges, (*count + 1) * sizeof **ranges);
    if (next == NULL) {
        return false;
    }
    *ranges = next;
    next[(*count)++] = (ScoopPlatformImageRange){start, end};
    return true;
}

static int compare_range(const void *left, const void *right) {
    uintptr_t a = ((const ScoopPlatformImageRange *)left)->start;
    uintptr_t b = ((const ScoopPlatformImageRange *)right)->start;
    return (a > b) - (a < b);
}

void scoop_elf_dispose_metadata_images(ScoopPlatformMetadataImages *images) {
    if (images == NULL) {
        return;
    }
    free((void *)images->ranges);
    free((void *)images->images);
    *images = (ScoopPlatformMetadataImages){0};
}

static bool collect_ranges(const ScoopElfLoadedImageView *view,
                           ScoopPlatformMetadataImages *images, ScoopStackMapImage *stackmaps) {
    ScoopPlatformImageRange *ranges = NULL;
    for (size_t index = 0; index < view->program_header_count; ++index) {
        const Elf64_Phdr *header = &view->program_headers[index];
        if (header->p_type != PT_LOAD || header->p_memsz == 0) {
            continue;
        }
        uintptr_t start, end;
        if (!scoop_elf_loaded_segment_range(view, header, &start, &end) ||
            !append_range(&ranges, &images->range_count, start, end)) {
            free(ranges);
            images->range_count = 0;
            return false;
        }
        if (header->p_flags & PF_X) {
            if (stackmaps->text_start == 0 || start < stackmaps->text_start) {
                stackmaps->text_start = start;
            }
            if (end > stackmaps->text_end) {
                stackmaps->text_end = end;
            }
        }
    }
    images->ranges = ranges;
    return true;
}

static bool validate_ranges(ScoopPlatformMetadataImages *images,
                            const ScoopStackMapImage *stackmaps) {
    if (images->range_count == 0) {
        return false;
    }
    qsort((void *)images->ranges, images->range_count, sizeof *images->ranges, compare_range);
    for (size_t index = 1; index < images->range_count; ++index) {
        if (images->ranges[index - 1].end > images->ranges[index].start) {
            return false;
        }
    }
    return stackmaps->text_start != 0 && stackmaps->text_end > stackmaps->text_start &&
           scoop_image_range_contains(images, (const void *)stackmaps->text_start,
                                      stackmaps->text_end - stackmaps->text_start, 1);
}

static bool attach_stackmaps(const uint8_t *stackmaps_start, const uint8_t *stackmaps_end,
                             ScoopPlatformMetadataImages *images, ScoopStackMapImage stackmaps) {
    uintptr_t start = (uintptr_t)stackmaps_start;
    uintptr_t end = (uintptr_t)stackmaps_end;
    if (start > end) {
        return false;
    }
    if (start == end) {
        return true;
    }
    stackmaps.section = stackmaps_start;
    stackmaps.section_size = end - start;
    if (!scoop_image_range_contains(images, stackmaps.section, stackmaps.section_size, 8)) {
        return false;
    }
    ScoopStackMapImage *owned = malloc(sizeof *owned);
    if (owned == NULL) {
        return false;
    }
    *owned = stackmaps;
    images->images = owned;
    images->count = 1;
    return true;
}

bool scoop_elf_metadata_images_from_view(const ScoopElfLoadedImageView *view,
                                         const uint8_t *stackmaps_start,
                                         const uint8_t *stackmaps_end,
                                         ScoopPlatformMetadataImages *images) {
    if (view == NULL || images == NULL ||
        (view->program_header_count != 0 && view->program_headers == NULL)) {
        return false;
    }
    *images = (ScoopPlatformMetadataImages){0};
    ScoopStackMapImage stackmaps = {0};
    if (!collect_ranges(view, images, &stackmaps) || !validate_ranges(images, &stackmaps) ||
        !attach_stackmaps(stackmaps_start, stackmaps_end, images, stackmaps)) {
        scoop_elf_dispose_metadata_images(images);
        return false;
    }
    return true;
}
