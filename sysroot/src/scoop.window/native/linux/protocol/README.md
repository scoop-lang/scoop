# Wayland protocol bindings

These files were generated with wayland-scanner 1.24.0 from wayland-protocols
1.47. Their upstream copyright and permissive license notices are retained in
the generated C files and headers. Upstream source:
https://gitlab.freedesktop.org/wayland/wayland-protocols

| Generated basename | XML in wayland-protocols |
| --- | --- |
| viewporter | stable/viewporter/viewporter.xml |
| fractional-scale-v1 | staging/fractional-scale/fractional-scale-v1.xml |
| xdg-activation-v1 | staging/xdg-activation/xdg-activation-v1.xml |
| text-input-unstable-v3 | unstable/text-input/text-input-unstable-v3.xml |

Generate each pair from its corresponding XML:

```sh
wayland-scanner client-header "$protocol_xml" "$generated_basename.h"
wayland-scanner private-code "$protocol_xml" "$generated_basename.c"
```

The bindings are compiled as ordinary native sources. Applications do not run
wayland-scanner, download protocol definitions, or generate code during linking.
libdecor owns the xdg-shell and decoration bindings used by the window frame.
