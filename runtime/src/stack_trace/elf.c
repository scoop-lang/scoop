#include "elf.h"

#include <string.h>

static size_t align_note(size_t size) { return (size + 3) & ~(size_t)3; }

void scoop_trace_elf_build_id(const ScoopElfLoadedImageView *image,
                              ScoopTraceModule *module) {
    if (image == NULL || module == NULL ||
        (image->program_header_count != 0 && image->program_headers == NULL)) {
        return;
    }
    for (size_t index = 0; index < image->program_header_count; ++index) {
        const Elf64_Phdr *segment = &image->program_headers[index];
        if (segment->p_type != PT_NOTE) {
            continue;
        }
        uintptr_t start, end;
        if (!scoop_elf_loaded_segment_range(image, segment, &start, &end)) {
            continue;
        }
        const unsigned char *cursor = (const void *)start;
        size_t remaining = end - start;
        while (remaining >= sizeof(Elf64_Nhdr)) {
            Elf64_Nhdr note;
            memcpy(&note, cursor, sizeof note);
            cursor += sizeof note;
            remaining -= sizeof note;
            size_t name = align_note(note.n_namesz);
            size_t value = align_note(note.n_descsz);
            if (name > remaining || value > remaining - name) {
                break;
            }
            if (note.n_type == NT_GNU_BUILD_ID && note.n_namesz == 4 &&
                memcmp(cursor, "GNU", 4) == 0 && note.n_descsz <= SCOOP_TRACE_MAX_ID) {
                module->id_kind = 2;
                module->id_length = note.n_descsz;
                memcpy(module->id, cursor + name, note.n_descsz);
                return;
            }
            cursor += name + value;
            remaining -= name + value;
        }
    }
}
