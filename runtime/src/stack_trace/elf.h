#ifndef SCOOP_STACK_TRACE_ELF_H
#define SCOOP_STACK_TRACE_ELF_H

#include "internal.h"
#include "../platform/image/elf.h"

void scoop_trace_elf_build_id(const ScoopElfLoadedImageView *image,
                              ScoopTraceModule *module);

#endif /* SCOOP_STACK_TRACE_ELF_H */
