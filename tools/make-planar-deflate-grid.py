#!/usr/bin/env python3
"""Writes the offline GeoTIFF grids of the network tests: planar (PlanarConfiguration 2), three
float32 bands, Deflate (Compression 8) and the floating point predictor (Predictor 3), as the grids
of `cdn.proj.org` are (`nz_linz_nzgd2000-c120100904-grid01.tif`).

Two layouts. The default has one strip per band, the layout of the CDN grid. `--tiled` has 32 by 32
tiles, and a size that is not a multiple of 32, so that the tiles at the right and bottom edges are
partial, and padded in the file as libtiff pads them.

The layout is the header, the tag values, the IFD, the offset arrays, then the chunks. A reader
that creates the grid needs only the first 16 KiB, and the chunks lie beyond it.

The predictor is libtiff's `fpDiff` (`tif_predict.c`) applied to each row of a chunk, with a stride
of one because the bands are planar. The output is the same bytes on every run:

    tools/make-planar-deflate-grid.py data/tests/planar_deflate_predictor3.tif
    tools/make-planar-deflate-grid.py --tiled data/tests/planar_deflate_predictor3_tiled.tif
"""

import struct
import sys
import zlib
from collections import namedtuple

Layout = namedtuple("Layout", "width height tile")
# The strips of the CDN grid: one strip per band, the whole image wide.
STRIPS = Layout(width=128, height=96, tile=None)
# Tiles of 32 by 32 pixels; 100 by 70 pixels leaves partial tiles at the right and bottom edges.
TILES = Layout(width=100, height=70, tile=(32, 32))

WEST = 10.0
NORTH = 60.0
STEP = 0.25
# The description of each band, as GDAL writes them (`DESCRIPTION` items). The vertical shift
# of `vgridshift` is the band described `vertical_offset`, the third one here.
DESCRIPTIONS = ["east", "north", "vertical_offset"]
BANDS = 3


def band_value(band, col, row):
    """The sample of `band` at pixel `(col, row)`: a slope, and a small deterministic noise
    so that the chunks are not trivially compressible."""
    noise = ((col * 7919 + row * 104729 + band * 1299709) % 9973) / 9973.0 * 0.02
    return 0.25 + 0.5 * band + 0.01 * col - 0.02 * row + noise


def fp_predict_row(samples):
    """The floating point predictor of one row: libtiff's `fpDiff` with a stride of one."""
    count = len(samples)
    raw = struct.pack("<%df" % count, *samples)
    planes = bytearray(4 * count)
    for index in range(count):
        for byte in range(4):
            planes[(4 - byte - 1) * count + index] = raw[index * 4 + byte]
    for index in range(len(planes) - 1, 0, -1):
        planes[index] = (planes[index] - planes[index - 1]) & 0xFF
    return bytes(planes)


def compressed_chunk(layout, band, left, top, chunk_width, chunk_height):
    """One chunk of `band`: the pixels from `(left, top)`, `chunk_width` by `chunk_height`, the
    pixels outside the image being zero, as libtiff pads the edge tiles."""
    rows = bytearray()
    for row in range(top, top + chunk_height):
        samples = []
        for col in range(left, left + chunk_width):
            inside = col < layout.width and row < layout.height
            samples.append(band_value(band, col, row) if inside else 0.0)
        rows += fp_predict_row(samples)
    return zlib.compress(bytes(rows))


def chunks_of(layout):
    """The compressed chunks, in the file's order: the bands one after the other, and the chunks
    of a band in row order."""
    out = []
    for band in range(BANDS):
        if layout.tile is None:
            out.append(compressed_chunk(layout, band, 0, 0, layout.width, layout.height))
            continue
        tile_w, tile_h = layout.tile
        for top in range(0, layout.height, tile_h):
            for left in range(0, layout.width, tile_w):
                out.append(compressed_chunk(layout, band, left, top, tile_w, tile_h))
    return out


def entry(tag, kind, count, value):
    return struct.pack("<HHII", tag, kind, count, value)


def build(layout):
    chunks = chunks_of(layout)
    # GTModelType 2 (geographic), RasterType 1 (PixelIsArea), GeographicType 4326 (WGS 84).
    keys = [1, 1, 0, 3, 1024, 0, 1, 2, 1025, 0, 1, 1, 2048, 0, 1, 4326]
    metadata = "<GDALMetadata>" + "".join(
        '<Item name="DESCRIPTION" sample="%d">%s</Item>' % (index, text)
        for index, text in enumerate(DESCRIPTIONS)
    ) + "</GDALMetadata>\0"
    metadata_bytes = metadata.encode("ascii")

    # The values that do not fit in an IFD entry, after the header, each at an even offset.
    values = bytearray()
    positions = {}

    def place(name, data):
        if len(values) % 2:
            values.append(0)
        positions[name] = 8 + len(values)
        values.extend(data)

    place("scale", struct.pack("<3d", STEP, STEP, 0.0))
    place("tie", struct.pack("<6d", 0.0, 0.0, 0.0, WEST, NORTH, 0.0))
    place("keys", struct.pack("<%dH" % len(keys), *keys))
    place("bits", struct.pack("<3H", 32, 32, 32))
    place("format", struct.pack("<3H", 3, 3, 3))
    place("metadata", metadata_bytes)

    if len(values) % 2:
        values.append(0)
    count = len(chunks)
    tag_count = 17 if layout.tile else 16
    ifd_at = 8 + len(values)
    ifd_size = 2 + 12 * tag_count + 4
    offsets_at = ifd_at + ifd_size
    counts_at = offsets_at + 4 * count
    chunks_at = counts_at + 4 * count

    offsets = []
    position = chunks_at
    for chunk in chunks:
        offsets.append(position)
        position += len(chunk)
    byte_counts = [len(chunk) for chunk in chunks]

    tags = [
        entry(256, 4, 1, layout.width),
        entry(257, 4, 1, layout.height),
        entry(258, 3, 3, positions["bits"]),
        entry(259, 3, 1, 8),
        entry(262, 3, 1, 1),
    ]
    if layout.tile is None:
        tags += [
            entry(273, 4, count, offsets_at),
            entry(277, 3, 1, BANDS),
            entry(278, 4, 1, layout.height),
            entry(279, 4, count, counts_at),
        ]
    else:
        tags += [
            entry(277, 3, 1, BANDS),
            entry(322, 3, 1, layout.tile[0]),
            entry(323, 3, 1, layout.tile[1]),
            entry(324, 4, count, offsets_at),
            entry(325, 4, count, counts_at),
        ]
    tags += [
        entry(284, 3, 1, 2),
        entry(317, 3, 1, 3),
        entry(339, 3, 3, positions["format"]),
        entry(33550, 12, 3, positions["scale"]),
        entry(33922, 12, 6, positions["tie"]),
        entry(34735, 3, len(keys), positions["keys"]),
        entry(42112, 2, len(metadata_bytes), positions["metadata"]),
    ]
    assert len(tags) == tag_count

    out = bytearray(b"II" + struct.pack("<HI", 42, ifd_at))
    out += values
    out += struct.pack("<H", tag_count) + b"".join(tags) + struct.pack("<I", 0)
    out += struct.pack("<%dI" % count, *offsets)
    out += struct.pack("<%dI" % count, *byte_counts)
    for chunk in chunks:
        out += chunk
    assert len(out) == chunks_at + sum(byte_counts)
    return bytes(out)


def main():
    arguments = sys.argv[1:]
    tiled = arguments[:1] == ["--tiled"]
    if tiled:
        arguments = arguments[1:]
    if len(arguments) != 1:
        sys.exit("usage: make-planar-deflate-grid.py [--tiled] <output.tif>")
    with open(arguments[0], "wb") as file:
        file.write(build(TILES if tiled else STRIPS))


if __name__ == "__main__":
    main()
