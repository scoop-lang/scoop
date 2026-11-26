/* LLD uses the explicit RELRO bounds for custom output sections. */
SECTIONS
{
  . = DATA_SEGMENT_ALIGN(CONSTANT(MAXPAGESIZE), CONSTANT(COMMONPAGESIZE));
  .data.rel.ro.scoop : ALIGN(8)
  {
    KEEP(*(.data.rel.ro.scoop .data.rel.ro.scoop.*))
  }
  . = DATA_SEGMENT_RELRO_END(0, .);
}
INSERT BEFORE .dynamic;
