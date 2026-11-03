#define _GNU_SOURCE
#include "elf.h"

#include <link.h>
#include <unistd.h>

#if !defined(__linux__) || UINTPTR_MAX != UINT64_MAX
#error "the Linux stack-trace loader requires 64-bit Linux"
#endif

_Static_assert(sizeof(ElfW(Phdr)) == sizeof(Elf64_Phdr), "Linux ELF program header size");
_Static_assert(_Alignof(ElfW(Phdr)) == _Alignof(Elf64_Phdr),
               "Linux ELF program header alignment");

typedef struct ModuleQuery {
    ScoopTraceFrames *frames;
    ScoopTraceLoadedModule *modules;
    uint32_t count;
    uint64_t generation;
} ModuleQuery;

static ScoopElfLoadedImageView loaded_image_view(const struct dl_phdr_info *image) {
    return (ScoopElfLoadedImageView){
        .load_bias = (uintptr_t)image->dlpi_addr,
        .program_headers = (const Elf64_Phdr *)image->dlpi_phdr,
        .program_header_count = image->dlpi_phnum,
    };
}

static const char *module_path(const struct dl_phdr_info *image, char executable[4096]) {
    const char *path = image->dlpi_name;
    if (path != NULL && path[0] != 0) {
        return path;
    }
    ssize_t length = readlink("/proc/self/exe", executable, 4095);
    if (length >= 0 && length < 4095) {
        executable[length] = 0;
        return executable;
    }
    return "/proc/self/exe";
}

static int visit_image(struct dl_phdr_info *image, size_t size, void *data) {
    (void)size;
    ModuleQuery *query = data;
    query->generation = image->dlpi_adds + image->dlpi_subs;
    ScoopElfLoadedImageView view = loaded_image_view(image);
    uint32_t module = SCOOP_TRACE_NO_MODULE;
    for (size_t segment_index = 0; segment_index < view.program_header_count;
         ++segment_index) {
        const Elf64_Phdr *segment = &view.program_headers[segment_index];
        uintptr_t low, high;
        if (segment->p_type != PT_LOAD ||
            !scoop_elf_loaded_segment_range(&view, segment, &low, &high)) {
            continue;
        }
        for (uint32_t index = 0; index < query->frames->count; ++index) {
            ScoopTraceFrame *frame = &query->frames->frames[index];
            if (frame->query_pc < low || frame->query_pc >= high ||
                frame->module != SCOOP_TRACE_NO_MODULE) {
                continue;
            }
            if (module == SCOOP_TRACE_NO_MODULE) {
                module = query->count++;
                ScoopTraceLoadedModule *found = &query->modules[module];
                found->identity.load_bias = image->dlpi_addr;
                found->identity.image_base = low;
                scoop_trace_elf_build_id(&view, &found->identity);
                char executable[4096];
                found->path = scoop_trace_copy_path(module_path(image, executable));
            }
            frame->module = module;
            frame->link_pc = frame->query_pc - view.load_bias;
        }
    }
    return 0;
}

static int visit_generation(struct dl_phdr_info *image, size_t size, void *data) {
    (void)size;
    *(uint64_t *)data = image->dlpi_adds + image->dlpi_subs;
    return 1;
}

uint64_t scoop_trace_generation(void) {
    uint64_t generation = 0;
    dl_iterate_phdr(visit_generation, &generation);
    return generation;
}

uint32_t scoop_trace_modules(ScoopTraceFrames *frames, ScoopTraceLoadedModule *modules,
                             uint64_t *generation) {
    ModuleQuery query = {.frames = frames, .modules = modules};
    /* The platform loader keeps its image names and notes valid in callbacks. */
    dl_iterate_phdr(visit_image, &query);
    *generation = query.generation;
    return query.count;
}
