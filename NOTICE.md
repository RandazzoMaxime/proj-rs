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
  [`data/COPYING`](data/COPYING). Its content comes from the registries below.

## EPSG Geodetic Parameter Dataset

`data/proj.db`, embedded in the library, contains data from the EPSG Geodetic
Parameter Dataset, version v13.102 (2026-08-27, as its `metadata` table
records). The EPSG Dataset is owned by the International Association of Oil &
Gas Producers (IOGP) and is published at <https://epsg.org>.

Its use is subject to the EPSG Dataset Terms of Use,
<https://epsg.org/terms-of-use.html>. Anyone who passes this software on must
pass these terms on as well. In short:

- the EPSG Facilities are published by IOGP at no charge, and distribution for
  profit is forbidden; the data may be part of a commercial package only when
  the commercial value comes from the provider, not from the EPSG Dataset;
- ownership of the EPSG Dataset by IOGP must be acknowledged in any
  publication or transmission of it, modified or not;
- the data are provided "as is", without warranty of any kind; neither IOGP
  nor its members warrant their accuracy or accept liability for their use;
- only the modifications listed in the Terms of Use may still be attributed to
  the EPSG Dataset.

The Terms of Use are the reference; this summary does not replace them.

## Other registries in `data/proj.db`

`data/proj.db` also holds, as PROJ 9.9.0 ships them:

- ESRI projection engine definitions, ArcGIS Pro 3.6 (2025-12-01);
- the IGNF geodetic registry of the French national mapping agency, version
  3.1.0 (2019-05-24), from
  <https://raw.githubusercontent.com/rouault/proj-resources/master/IGNF.v3.1.0.xml>;
- the Nordic transformations of the Nordic Geodetic Commission (NKG), version
  1.0.w (2025-02-13), from <https://github.com/NordicGeodesy/NordicTransformations>.

Their terms are those of their publishers.

## Exact Transverse Mercator

The exact Transverse Mercator takes its series from Knud Poder and Karsten
Engsager.

## Geodesics

`src/geodesic/` is Charles Karney's C geodesic library, the same one PROJ 9.9
calls. Copyright (c) Charles Karney, MIT/X11 license; see the header of
[`src/geodesic/geodesic.c`](src/geodesic/geodesic.c) and
<https://geographiclib.sourceforge.io/>. The inverse of the conformal latitude
follows GeographicLib, as shipped in PROJ.

## georust `proj` tests

`tests/georust_parity.rs` runs the unit tests of the georust
[`proj`](https://github.com/georust/proj) crate 0.31.0 (`src/proj.rs` and
`src/geo_types.rs`) against this crate's facade. Inputs and expected values are
georust's, copyright the GeoRust contributors, MIT OR Apache-2.0.
