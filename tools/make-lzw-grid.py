#!/usr/bin/env python3
"""Writes the compressed grids of the network and local tests: the same samples as
`planar_deflate_predictor3.tif` (or `--tiled`: `planar_deflate_predictor3_tiled.tif`), with LZW
(`Compression` 5, the default) or PackBits (`--codec packbits`, `Compression` 32773) instead of
Deflate. Three planar float32 bands, one strip per band, and the floating point predictor
(`Predictor` 3), as `make-planar-deflate-grid.py` writes them. `--big-endian` writes the file in
big-endian order.

The samples are the same doubles, packed to float32 the same way, so the values libproj 9.9.0
measured on the Deflate grid are the values of this one. The LZW encoder is TIFF's: codes are
MSB-first, the code width grows one code early, and a `ClearCode` starts the stream and restarts
the table when it is nearly full.

The floating point predictor takes the byte planes of each sample, most significant first, whatever
the file order: a big-endian file has the same compressed chunks as the little-endian one, and only
its header and its byte order differ. libproj 9.9.0 (libtiff) reads both with the Deflate values.

    tools/make-lzw-grid.py data/tests/lzw_predictor3_planar.tif
    tools/make-lzw-grid.py --tiled data/tests/lzw_predictor3_planar_tiled.tif
    tools/make-lzw-grid.py --big-endian data/tests/lzw_be_predictor3_planar.tif
    tools/make-lzw-grid.py --big-endian --tiled data/tests/lzw_be_predictor3_planar_tiled.tif
    tools/make-lzw-grid.py --codec packbits --predictor 1 --tiled data/tests/packbits_planar_tiled.tif
    tools/make-lzw-grid.py --codec packbits --tiled data/tests/packbits_predictor3_planar_tiled.tif
"""

import argparse
import importlib.util
import os
import struct
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "make_planar_deflate_grid", os.path.join(_HERE, "make-planar-deflate-grid.py")
)
planar = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(planar)

CLEAR = 256
END = 257
FIRST = 258
TABLE_LIMIT = 4096


def width_for_code(index):
    """The width of the code at position `index` (1 for the first code after a clear). The decoder
    widens the code after it has added the entry that makes its table reach 511, 1023 or 2047."""
    if index == 1:
        return 9
    table = 256 + index
    if table >= 2047:
        return 12
    if table >= 1023:
        return 11
    if table >= 511:
        return 10
    return 9


class BitWriter:
    def __init__(self):
        self.out = bytearray()
        self.acc = 0
        self.bits = 0

    def write(self, value, width):
        self.acc = (self.acc << width) | value
        self.bits += width
        while self.bits >= 8:
            self.bits -= 8
            self.out.append((self.acc >> self.bits) & 0xFF)
        self.acc &= (1 << self.bits) - 1

    def finish(self):
        if self.bits:
            self.out.append((self.acc << (8 - self.bits)) & 0xFF)
        return bytes(self.out)


def lzw_encode(data):
    writer = BitWriter()
    table = {bytes([value]): value for value in range(256)}
    next_code = FIRST
    index = 0
    writer.write(CLEAR, 9)
    current = b""
    for byte in data:
        candidate = current + bytes([byte])
        if candidate in table:
            current = candidate
            continue
        index += 1
        writer.write(table[current], width_for_code(index))
        if next_code < TABLE_LIMIT - 2:
            table[candidate] = next_code
            next_code += 1
        current = bytes([byte])
        if next_code >= TABLE_LIMIT - 2:
            index += 1
            writer.write(CLEAR, width_for_code(index))
            table = {bytes([value]): value for value in range(256)}
            next_code = FIRST
            index = 0
    if current:
        index += 1
        writer.write(table[current], width_for_code(index))
    index += 1
    writer.write(END, width_for_code(index))
    return writer.finish()


def packbits_encode(data):
    """PackBits (Apple's run-length code, TIFF `Compression` 32773): a run of 2 to 128 equal bytes
    is `1 - n` and the byte; a literal of 1 to 128 bytes is `n - 1` and the bytes."""
    out = bytearray()
    index = 0
    while index < len(data):
        run = 1
        while index + run < len(data) and run < 128 and data[index + run] == data[index]:
            run += 1
        if run >= 2:
            out.append((257 - run) & 0xFF)
            out.append(data[index])
            index += run
            continue
        start = index
        while index < len(data) and index - start < 128:
            if index + 1 < len(data) and data[index] == data[index + 1]:
                break
            index += 1
        out.append(index - start - 1)
        out += data[start:index]
    return bytes(out)


def plain_row(samples):
    """A row without predictor: the little-endian float32 samples."""
    return struct.pack("<%df" % len(samples), *samples)


def compressed_chunk(layout, band, left, top, chunk_width, chunk_height, codec, predictor):
    """The compressed bytes of one chunk, the same samples as `make-planar-deflate-grid.py`."""
    rows = bytearray()
    for row in range(top, top + chunk_height):
        samples = []
        for col in range(left, left + chunk_width):
            inside = col < layout.width and row < layout.height
            samples.append(planar.band_value(band, col, row) if inside else 0.0)
        rows += planar.fp_predict_row(samples) if predictor == 3 else plain_row(samples)
    if codec == "packbits":
        return packbits_encode(bytes(rows))
    return lzw_encode(bytes(rows))


def chunks_of(layout, codec, predictor):
    out = []
    for band in range(planar.BANDS):
        if layout.tile is None:
            out.append(
                compressed_chunk(layout, band, 0, 0, layout.width, layout.height, codec, predictor)
            )
            continue
        tile_w, tile_h = layout.tile
        for top in range(0, layout.height, tile_h):
            for left in range(0, layout.width, tile_w):
                out.append(
                    compressed_chunk(layout, band, left, top, tile_w, tile_h, codec, predictor)
                )
    return out


def build(layout, codec, predictor, big_endian):
    order = ">" if big_endian else "<"
    chunks = chunks_of(layout, codec, predictor)
    keys = [1, 1, 0, 3, 1024, 0, 1, 2, 1025, 0, 1, 1, 2048, 0, 1, 4326]
    metadata = "<GDALMetadata>" + "".join(
        '<Item name="DESCRIPTION" sample="%d">%s</Item>' % (index, text)
        for index, text in enumerate(planar.DESCRIPTIONS)
    ) + "</GDALMetadata>\0"
    metadata_bytes = metadata.encode("ascii")

    values = bytearray()
    positions = {}

    def place(name, data):
        if len(values) % 2:
            values.append(0)
        positions[name] = 8 + len(values)
        values.extend(data)

    place("scale", struct.pack(order + "3d", planar.STEP, planar.STEP, 0.0))
    place("tie", struct.pack(order + "6d", 0.0, 0.0, 0.0, planar.WEST, planar.NORTH, 0.0))
    place("keys", struct.pack(order + "%dH" % len(keys), *keys))
    place("bits", struct.pack(order + "3H", 32, 32, 32))
    place("format", struct.pack(order + "3H", 3, 3, 3))
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
    compression = 32773 if codec == "packbits" else 5

    def entry(tag, kind, entry_count, value):
        # A SHORT value that fits one entry sits at the start of the 4-byte field, which is its
        # high half in a big-endian file.
        if kind == 3 and entry_count == 1 and big_endian:
            value <<= 16
        return struct.pack(order + "HHII", tag, kind, entry_count, value)

    tags = [
        entry(256, 4, 1, layout.width),
        entry(257, 4, 1, layout.height),
        entry(258, 3, 3, positions["bits"]),
        entry(259, 3, 1, compression),
        entry(262, 3, 1, 1),
    ]
    if layout.tile is None:
        tags += [
            entry(273, 4, count, offsets_at),
            entry(277, 3, 1, planar.BANDS),
            entry(278, 4, 1, layout.height),
            entry(279, 4, count, counts_at),
        ]
    else:
        tags += [
            entry(277, 3, 1, planar.BANDS),
            entry(322, 3, 1, layout.tile[0]),
            entry(323, 3, 1, layout.tile[1]),
            entry(324, 4, count, offsets_at),
            entry(325, 4, count, counts_at),
        ]
    tags += [
        entry(284, 3, 1, 2),
        entry(317, 3, 1, predictor),
        entry(339, 3, 3, positions["format"]),
        entry(33550, 12, 3, positions["scale"]),
        entry(33922, 12, 6, positions["tie"]),
        entry(34735, 3, len(keys), positions["keys"]),
        entry(42112, 2, len(metadata_bytes), positions["metadata"]),
    ]
    assert len(tags) == tag_count

    magic = b"MM" if big_endian else b"II"
    out = bytearray(magic + struct.pack(order + "HI", 42, ifd_at))
    out += values
    out += struct.pack(order + "H", tag_count) + b"".join(tags) + struct.pack(order + "I", 0)
    out += struct.pack(order + "%dI" % count, *offsets)
    out += struct.pack(order + "%dI" % count, *byte_counts)
    for chunk in chunks:
        out += chunk
    assert len(out) == chunks_at + sum(byte_counts)
    return bytes(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--tiled", action="store_true", help="32 by 32 tiles on a 100 by 70 grid")
    parser.add_argument("--big-endian", action="store_true", help="write the file big-endian")
    parser.add_argument(
        "--codec", choices=["lzw", "packbits"], default="lzw", help="the compression (default lzw)"
    )
    parser.add_argument(
        "--predictor", type=int, choices=[1, 3], default=3, help="1 for none, 3 (default) for float"
    )
    parser.add_argument("output")
    arguments = parser.parse_args()
    layout = planar.TILES if arguments.tiled else planar.STRIPS
    with open(arguments.output, "wb") as file:
        file.write(build(layout, arguments.codec, arguments.predictor, arguments.big_endian))


if __name__ == "__main__":
    sys.exit(main())
