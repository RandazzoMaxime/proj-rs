# Third-party notices

PROJ-RS is released under the GNU Affero General Public License v3.0 in [LICENSE](LICENSE), and
under a commercial license on request ([COMMERCIAL-LICENSE.md](https://github.com/helix-aero/proj-rs/blob/main/COMMERCIAL-LICENSE.md)); versions up
to 1.2.6 were released under the MIT license. It ports, and in places ships, work by other people.
This file lists it.

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

`data/proj.db` also holds, as PROJ 9.9.0 ships them, three registries under
their own open licences. The sources were checked on 2026-11-15.

- **ESRI** projection engine definitions, ArcGIS Pro 3.6 (2025-12-01).
  Copyright 2010-2026 Esri, licensed under the Apache License, Version 2.0, as
  stated in <https://github.com/Esri/projection-engine-db-doc> (README,
  « Licensing », and `license.txt`; the repository has no `NOTICE` file). A
  copy of the licence is in
  [`data/licenses/Apache-2.0-Esri.txt`](data/licenses/Apache-2.0-Esri.txt).
  The data are converted into the tables of `proj.db` by PROJ's
  `build_db_from_esri.py`, unchanged in this crate.
- **IGNF** geodetic registry of the French national mapping agency (IGN),
  version 3.1.0 (2019-05-24), from
  <https://raw.githubusercontent.com/rouault/proj-resources/master/IGNF.v3.1.0.xml>.
  The file carries no licence of its own. Since 1 January 2021 the data IGN
  publishes without third-party rights are under the Licence Ouverte / Open
  Licence 2.0 (Etalab), by decision of its board of 11 December 2020; the IGNF
  registry is such data. Attribution: « Source : IGN — registre IGNF 3.1.0,
  2019-05-24 ». Licence text:
  <https://www.etalab.gouv.fr/licence-ouverte-open-licence/>.
- **NKG** Nordic transformations of the Nordic Geodetic Commission, version
  1.0.w (2025-02-13), from <https://github.com/NordicGeodesy/NordicTransformations>,
  under the Creative Commons Attribution 4.0 International licence, as its
  `LICENSE` states. Attribution: « Nordic Geodetic Commission (NKG),
  NordicTransformations 1.0.w ». A copy of the licence is in
  [`data/licenses/CC-BY-4.0-NKG.txt`](data/licenses/CC-BY-4.0-NKG.txt).

None of these terms forbids a commercial use. The EPSG Terms of Use above do
forbid distributing the EPSG Dataset for profit.

## Exact Transverse Mercator

The exact Transverse Mercator takes its series from Knud Poder and Karsten
Engsager.

## Geodesics

`src/geodesic.rs` is a port to Rust of Charles Karney's C geodesic library, the
same one PROJ 9.9 calls (`geodesic.c`, `geodesic.h`). The Rust code follows the C
function by function and keeps the order of its operations. Copyright (c)
Charles Karney (2012-2025), MIT/X11 license; see the header of
[`src/geodesic.rs`](src/geodesic.rs) and <https://geographiclib.sourceforge.io/>.
The coefficient tables and the algorithm are Karney's; the license of the original
file is:

> Copyright (c) Charles Karney (2012-2025) <karney@alum.mit.edu> and licensed
> under the MIT/X11 License. For more information, see
> https://geographiclib.sourceforge.io/

The inverse of the conformal latitude follows GeographicLib, as shipped in PROJ.

## georust `proj` tests

`tests/georust_parity.rs` runs the unit tests of the georust
[`proj`](https://github.com/georust/proj) crate 0.31.0 (`src/proj.rs` and
`src/geo_types.rs`) against this crate's facade. Inputs and expected values are
georust's, copyright the GeoRust contributors, MIT OR Apache-2.0.

## Vector math (Cephes)

`src/simd/math.rs` evaluates `atan`, `atan2` and `log` with the rational
approximations of the Cephes Mathematical Library by Stephen L. Moshier
(copyright 1984-2000). The coefficients were taken from the
[`wide`](https://crates.io/crates/wide) crate (Apache-2.0, MIT or Zlib), which
has them from Agner Fog's vector class library (Apache-2.0). Cephes is
redistributed under the BSD 3-clause licence with its author's permission, as
in SciPy (`scipy/special/cephes`). The other functions of that file follow
fdlibm, copyright (c) 1993 Sun Microsystems, whose notice permits use, copy,
modification and distribution provided the notice is kept.
