#!/usr/bin/env python3
"""Copies a little-endian classic TIFF and sets the `Compression` tag (259) of each image.

The samples are not re-encoded: the bytes stay those of the source, so a JPEG or CCITT grid made
this way has the Deflate grid's chunks under another compression. That is what the fixtures need:
libtiff refuses to decode them, and PROJ opens the grid anyway (libproj 9.9.0 gives `inf` for its
values, measured with `cct -d 17 +proj=vgridshift +grids=<file> +multiplier=1`).

    tools/patch-compression-tag.py data/tests/planar_deflate_predictor3.tif \
        data/tests/planar_deflate_predictor3_jpeg.tif 7
    tools/patch-compression-tag.py data/tests/planar_deflate_predictor3.tif \
        data/tests/planar_deflate_predictor3_ccitt.tif 2
"""

import struct
import sys

COMPRESSION = 259


def main(source, target, value):
    data = bytearray(open(source, "rb").read())
    if data[:4] != b"II*\x00":
        sys.exit("a little-endian classic TIFF is expected")
    patched = 0
    ifd = struct.unpack_from("<I", data, 4)[0]
    while ifd:
        count = struct.unpack_from("<H", data, ifd)[0]
        for index in range(count):
            entry = ifd + 2 + 12 * index
            tag, kind, n = struct.unpack_from("<HHI", data, entry)
            if tag == COMPRESSION:
                if kind != 3 or n != 1:
                    sys.exit("Compression is expected as one SHORT")
                struct.pack_into("<H", data, entry + 8, value)
                patched += 1
        ifd = struct.unpack_from("<I", data, ifd + 2 + 12 * count)[0]
    if patched == 0:
        sys.exit("no Compression tag")
    open(target, "wb").write(bytes(data))


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]))
