# Third-party notices

PROJ-RS is released under the MIT license in [LICENSE](LICENSE). It ports, and
in places ships, work by other people. This file lists it.

## PROJ

The projection algorithms ported from [PROJ](https://github.com/OSGeo/PROJ)
remain covered by PROJ's MIT license. Copyright (c) the PROJ authors.

- `tests/gie/` is the PROJ 9.9.0 `test/gie` suite and its test grids,
  vendored unchanged. PROJ's license text is in
  [`tests/gie/PROJ-COPYING`](tests/gie/PROJ-COPYING).
- `data/proj.db` is built from the PROJ 9.9.0 SQL. Its notice is in
  [`data/COPYING`](data/COPYING).

## Exact Transverse Mercator

The exact Transverse Mercator takes its series from Knud Poder and Karsten
Engsager.

## Geodesics

`src/geodesic/` is Charles Karney's C geodesic library, the same one PROJ 9.9
calls. Copyright (c) Charles Karney, MIT/X11 license; see the header of
[`src/geodesic/geodesic.c`](src/geodesic/geodesic.c) and
<https://geographiclib.sourceforge.io/>. The inverse of the conformal latitude
follows GeographicLib, as shipped in PROJ.
