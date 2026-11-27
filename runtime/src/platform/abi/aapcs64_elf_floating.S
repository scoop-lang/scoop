/* Scoop returns the tag and payload in x0/x1. The C helper fills their storage. */
.text
.macro SCOOP_TAGGED_FLOAT name, storage
.section .text.\name,"ax",%progbits
.p2align 2
.globl \name
.hidden \name
.type \name,%function
\name:
    .cfi_startproc
    stp x29, x30, [sp, #-32]!
    .cfi_def_cfa_offset 32
    .cfi_offset x29, -32
    .cfi_offset x30, -24
    mov x29, sp
    mov x1, x0
    add x0, sp, #16
    bl \storage
    ldp x0, x1, [sp, #16]
    ldp x29, x30, [sp], #32
    .cfi_def_cfa_offset 0
    .cfi_restore x29
    .cfi_restore x30
    ret
    .cfi_endproc
.size \name, .-\name
.endm

SCOOP_TAGGED_FLOAT scoop_rt_json_parse_float, scoop_rt_json_parse_float_storage
SCOOP_TAGGED_FLOAT scoop_rt_json_parse_double, scoop_rt_json_parse_double_storage

.section .note.GNU-stack,"",%progbits
