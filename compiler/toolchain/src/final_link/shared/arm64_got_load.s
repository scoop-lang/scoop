.text
.globl _main
.p2align 2
_main:
    mov x9, #37
Lpage:
    adrp x8, _value@GOTPAGE
    mov x0, x9
Lload:
    ldr x9, [x8, _value@GOTPAGEOFF]
    ldrb w9, [x9]
    sub w0, w0, w9
    ret
    .loh AdrpLdrGot Lpage, Lload

.const
.private_extern _value
_value:
    .byte 37
.subsections_via_symbols
