/* Pointer-bearing Scoop metadata is relocated before GNU_RELRO is applied. */
SECTIONS
{
  .data.rel.ro.scoop : ALIGN(8)
  {
    KEEP(*(.data.rel.ro.scoop .data.rel.ro.scoop.*))
  }
}
INSERT BEFORE .data.rel.ro;
