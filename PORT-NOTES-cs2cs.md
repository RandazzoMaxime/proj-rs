# PORT-NOTES cs2cs

Extended `src/bin/cs2cs.rs`. Also updated `tests/cli.rs` so the UTM checks pass `-d 8` (the C++ default for a projected result is `%.2f`, which is coarser than the old 1e-3 check). New file: `PORT-NOTES-cs2cs.md`.

`src/lib.rs` and `tests/gie/passing.txt` were not edited.

## Flags

Parsed the same way as PROJ 9.9 `src/apps/cs2cs.cpp`, including flags used by `test/cli/test_cs2cs_various.yaml` and `test/cli/test_cs2cs_ntv2.yaml`:

- `-d digits` sets `%.digitsf` for x, y, and z. `-d 8` prints eight decimals. It also turns DMS output off, as in PROJ.
- `-f` accepts the PROJ number formats (`%f`, `%e`, `%g`, width, precision, `+`, `0`).
- `-W0`..`-W8` is fixed-width DMS. `-w0`..`-w8` is compact DMS. `-W`, `-W9`, and `-W10` exit 1 with `-W argument missing or not in range [0,8]`.
- No `-d` or `-f`: a degree longitude/latitude result is DMS (`rtodms`, z `%.3f`); anything else is `%.2f` for x, y, and z.
- `-E` echoes the input through z, then a tab. `-I` swaps the two CRS strings. `-r` / `-s` swap input or output axes. `-e` replaces `*\t*`. `-t` sets the comment character. `-v` prints the two definitions. `-D` is accepted and ignored. `-l` exits 0; this port does not print the operation catalogue.
- `--area`, `--bbox`, `--authority`, `--accuracy`, `--only-best`, `--only-best=yes|no`, `--no-ballpark`, `--3d`, `--s_epoch`, `--t_epoch` are parsed. `--bbox` is stored on `Proj::new_known_crs`. The facade does not use that area to pick a grid, and it does not apply authority, accuracy, ballpark, only-best, 3D promotion, or epochs.
- `+to` still splits a PROJ string into source and target. A bare `SOURCE TARGET` pair still works. Extra arguments are input files. `-` is stdin. A leading UTF-8 BOM on the first line of a file is skipped.

## Behaviour the library does not match

`Proj::new_known_crs` stays longitude/latitude or easting/northing. EPSG axis order is not applied, so `EPSG:4326` is not latitude-first.

`Proj::convert` is two-dimensional. Height is copied from the input. A geocentric Z, `+vto_meter`, or a vertical grid is not applied. A failed conversion prints `*\t*` and `inf` for z.

A missing coordinate epoch warns only when `proj.db` records `frame_reference_epoch` on the datum (ITRF). WGS 84 ensemble is not treated as dynamic.
