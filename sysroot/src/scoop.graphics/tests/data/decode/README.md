# Image decode regression inputs

The four PNGs are locally generated 3 × 2 RGBA8 images, with valid chunk CRCs and
an IEND. `rgba.png` contains red/green/blue, transparent/white/black pixels with
alpha 255/128/64/0/255/128. `truncated-deflate.png` removes the last six bytes of
the IDAT zlib stream before recomputing the chunk CRC; `short-pixels.png` has only
one decompressed scanline; `bad-filter.png` replaces the first filter byte with
the invalid value 5. They distinguish incomplete/error-in-input codec results
from failures to recognize the header.

`rotated.jpg` is `../pixels.jpg` with a locally generated EXIF APP1 segment after
SOI. The little-endian TIFF IFD has a single SHORT Orientation tag (0x112) with
value 6. Its compressed pixels are identical; expected output is the original
2 × 2 image rotated clockwise.

No external images or additional codec dependencies are used.
