#ifndef SCOOP_RT_LSDA_ENCODING_H
#define SCOOP_RT_LSDA_ENCODING_H

#include <stdint.h>

/* The runtime build supplies the encodings of the selected backend. */
#if !defined(SCOOP_EH_LP_START_ENCODING) || !defined(SCOOP_EH_TTYPE_ENCODING) || \
    !defined(SCOOP_EH_CALL_SITE_ENCODING) || !defined(SCOOP_EH_OMITTED_TTYPE_ENCODING)
#error "Scoop runtime requires backend LSDA encodings"
#endif

enum {
    SCOOP_EH_DW_PE_ABSPTR = 0x00,
    SCOOP_EH_DW_PE_ULEB128 = 0x01,
    SCOOP_EH_DW_PE_PCREL_SDATA4_INDIRECT = 0x9b,
    SCOOP_EH_DW_PE_PCREL_SDATA8_INDIRECT = 0x9c,
    SCOOP_EH_DW_PE_OMIT = 0xff,
    SCOOP_EH_TYPE_ENTRY_SIZE =
        SCOOP_EH_TTYPE_ENCODING == SCOOP_EH_DW_PE_ABSPTR ? sizeof(uintptr_t) :
        SCOOP_EH_TTYPE_ENCODING == SCOOP_EH_DW_PE_PCREL_SDATA8_INDIRECT ? 8 : 4,
};

_Static_assert(SCOOP_EH_LP_START_ENCODING == SCOOP_EH_DW_PE_OMIT, "unsupported LSDA LPStart");
_Static_assert(SCOOP_EH_CALL_SITE_ENCODING == SCOOP_EH_DW_PE_ULEB128, "unsupported LSDA call site");
_Static_assert(SCOOP_EH_OMITTED_TTYPE_ENCODING == SCOOP_EH_DW_PE_OMIT, "unsupported LSDA omission");
_Static_assert(SCOOP_EH_TTYPE_ENCODING == SCOOP_EH_DW_PE_ABSPTR ||
                   SCOOP_EH_TTYPE_ENCODING == SCOOP_EH_DW_PE_PCREL_SDATA4_INDIRECT ||
                   SCOOP_EH_TTYPE_ENCODING == SCOOP_EH_DW_PE_PCREL_SDATA8_INDIRECT,
               "unsupported LSDA type table");

#endif
