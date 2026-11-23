#!/usr/bin/env python3
"""Pack data/proj.db into data/proj.pages: every page compressed on its own with zlib.

The embedded database is read one page at a time (src/projdb_exe.rs decompresses the pages a read
touches), so a page must not depend on the others. Format, little endian:
  magic b"PRJP", u32 page size, u32 page count, u64 database length,
  u32 offsets (page count + 1 of them, relative to the first byte after this table),
  then the zlib streams. The last page may be shorter than the page size.
Usage: pack_pages.py <proj.db> <proj.pages>. Standard library only.
"""
import struct
import sys
import zlib

PAGE = 4096


def pack(source, target):
    raw = open(source, "rb").read()
    streams = []
    for start in range(0, len(raw), PAGE):
        streams.append(zlib.compress(raw[start:start + PAGE], 9))
    offsets = [0]
    for stream in streams:
        offsets.append(offsets[-1] + len(stream))
    with open(target, "wb") as out:
        out.write(b"PRJP")
        out.write(struct.pack("<IIQ", PAGE, len(streams), len(raw)))
        out.write(struct.pack(f"<{len(offsets)}I", *offsets))
        for stream in streams:
            out.write(stream)
    print(f"{len(raw)} bytes -> {sum(map(len, streams)) + 8 + 16 + 4 * len(offsets)} bytes, {len(streams)} pages")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    pack(sys.argv[1], sys.argv[2])
