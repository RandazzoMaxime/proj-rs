# Port — `io.projparse_*` : WKT2 et chaînes PROJ de projections (lot io-4)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, groupe `io.projparse_*`, de
`io.projparse_longlat_complex` à `io.projparse_krovak_modified_czech`.
Côté crate : `tests/test_io.rs`, `src/iso19111/wkt.rs`.

Ce lot porte **18 `TEST()`** et corrige un écart d'export WKT2 dans
`src/iso19111/wkt.rs`. Il laisse **26 tests tentés** ouverts, avec la cause de
chacun. Il tente aussi quatre tests de pipeline, ouverts pour une cause
commune (section 3, groupe L).

## 0. Périmètre et base

- `docs/UNIT-TRIAGE.md` et `tests/docs_in_sync.rs` ne sont pas sur cette base
  (`main`, `ad1451a`). Ils sont dans le commit `65a7aad` de la branche
  `feat/pensive-allen-fkoqxo`, non fusionnée. Le lot suit donc l'ordre de la
  demande, pas l'ordre « core » du triage.
- `cargo test --test docs_in_sync` échoue : `no test target named docs_in_sync`.
  Les compteurs de `README.md` et de `ROADMAP.md` sont mis à jour à la main,
  du même delta que le ledger.

## 1. Écart corrigé dans `src` (≈ 10 lignes)

`export_method` et `export_parameter` (`src/iso19111/wkt.rs`) écrivaient
`ID["EPSG",0]` pour une méthode ou un paramètre sans code. Le `0` est la valeur
sentinelle de la base : PROJ n'écrit aucun `ID` pour ces objets. Une fonction
`has_code` exclut `""` et `"0"`.

Effet visible : `METHOD["Hotine Oblique Mercator Two Point Natural Origin"]`
sort sans `ID`, comme dans PROJ. Le test `io_projparse_omerc_tpno` le vérifie.
Aucune autre expectation de la suite ne contenait `EPSG",0]` (`grep`).

## 2. Portés (18)

| PROJ 9.9.0 `io.projparse_*` | PROJ-RS `tests/test_io.rs` | Ce qui est vérifié |
|---|---|---|
| `cea_spherical` | `io_projparse_cea_spherical` | aller-retour de la chaîne PROJ |
| `cea_ellipsoidal` | `io_projparse_cea_ellipsoidal` | méthode `Lambert Cylindrical Equal Area` ID 9835 |
| `lcc_as_lcc1sp` | `io_projparse_lcc_as_lcc1sp` | nom `Lambert Conic Conformal (1SP)` |
| `lcc_as_lcc1sp_variant_b` | `io_projparse_lcc_as_lcc1sp_variant_b` | nom `(1SP variant B)` |
| `lcc_as_lcc2sp` | `io_projparse_lcc_as_lcc2sp` | nom `(2SP)` |
| `lcc_as_lcc2sp_michigan` | `io_projparse_lcc_as_lcc2sp_michigan` | nom `(2SP Michigan)` |
| `merc_variant_B` | `io_projparse_merc_variant_b` | méthode `Mercator (variant B)` ID 9805, paramètre 1er parallèle |
| `omerc_nouoff` | `io_projparse_omerc_nouoff` | variante A, azimut, angle rectifié |
| `omerc_tpno` | `io_projparse_omerc_tpno` | nom de méthode sans `ID` (correction de la section 1) |
| `omerc_variant_b` | `io_projparse_omerc_variant_b` | variante B ID 9815, angle rectifié |
| `tmerc` | `io_projparse_tmerc` | WKT2 multiligne complet, égalité stricte avec la chaîne de PROJ |
| `krovak` | `io_projparse_krovak` | méthode `Krovak (North Orientated)` ID 1041 |
| `krovak_axis_swu` | `io_projparse_krovak_axis_swu` | méthode `Krovak` ID 9819 |
| `krovak_modified` | `io_projparse_krovak_modified` | méthode `Krovak Modified (North Orientated)` ID 1043 |
| `krovak_modified_axis_swu` | `io_projparse_krovak_modified_axis_swu` | méthode `Krovak Modified` ID 1042 |
| `merc_stere_polar_variant_A` | `io_projparse_merc_stere_polar_variant_a` | méthode `Polar Stereographic (variant A)` ID 9810 |
| `merc_stere_polar_variant_B` | `io_projparse_merc_stere_polar_variant_b` | méthode `Polar Stereographic (variant B)` ID 9829 |
| `merc_stere_polar_k_and_lat_ts` | `io_projparse_merc_stere_polar_k_and_lat_ts` | variante B, `Latitude of standard parallel` 90, chaîne PROJ exacte |

Les noms de test Rust sont ceux de PROJ en minuscules. Les tests cherchent
les sous-chaînes du `TEST()` source, sauf `tmerc`, où le WKT complet est
comparé. Le WKT est exporté en WKT2 2015, comme `WKTFormatter::create()` de
PROJ 9.9, avec `simulCurNodeHasId`.

## 3. Tentés, ouverts, avec la cause

Chaque cause a été vérifiée par un test qui échoue, puis retiré du fichier. Le
fichier ne contient que les tests qui passent.

### A. Import sans objet lié (`+towgs84`, `+nadgrids`, `+geoidgrids`)

`create_crs_from_proj_string` et `create_from_user_input` renvoient un
`GeographicCRS` nu, ou un `GeographicCRS` sans `+vunits`. Ils ne construisent
ni `Crs::Bound` ni `Crs::Compound`.

- `io.projparse_longlat_towgs84_3_terms`, `_7_terms`, `_7_terms_autocorrect`
- `io.projparse_longlat_nadgrids`, `_nadgrids_towgs84_ignored`
- `io.projparse_longlat_geoidgrids`, `_geoidgrids_vunits`

Fichier : `src/iso19111/projstring.rs`, import (`create_crs_from_proj_string`,
ligne 792). L'export (`bound.rs`, `projstring.rs`) sait déjà écrire ces
éléments, à partir d'un objet lié.

### B. Unités verticales

- `io.projparse_vunits`, `io.projparse_vto_meter` : `+vunits=ft +type=crs` et
  `+vto_meter=2 +type=crs` sans `+proj` ne donnent pas de `VerticalCRS`. Le
  premier appel renvoie `unsupported +proj= for a CRS`, le second
  `missing [` avec `create_from_user_input`.
- `io.projparse_longlat_vunits` : `+vunits=ft` est ignoré. Le CRS n'a que deux
  axes ; PROJ en écrit un troisième, `ellipsoidal height (h)`, `up`, `ORDER[3]`.

### C. Axes (`+axis=enu`, `neu`, `swu`)

- `io.projparse_longlat_axis_enu`, `_neu`, `_swu` : l'opération de
  `EPSG:4326` vers le CRS à axes permutés est une conversion `axisswap`.
  `create_operation_proj_string` refuse la paire (`not a projection of the
  source CRS`). `create_operations` la renvoie, mais
  `CoordinateOperation::export_to_proj_string_with_database`
  (`src/iso19111/transform_export.rs`) refuse une conversion sans sa paire de
  CRS. L'expression PROJ attendue n'est donc pas produite. La partie WKT
  (`AXIS["longitude",east,ORDER[1]`) n'a pas été vérifiée isolément.

### D. Projections non importées

- `io.projparse_aeqd_guam` : `+proj=aeqd +guam` renvoie `unsupported +proj=aeqd
  for a CRS`.
- `io.projparse_somerc` : `+proj=somerc` renvoie `unsupported +proj=somerc for
  a CRS`.
- `io.projparse_cea_spherical_on_ellipsoid` : `+R_A` renvoie `unsupported
  +proj=cea`.
- `io.projparse_cea_ellipsoidal_with_k_0` : `+k_0` sur `cea` renvoie la même
  erreur.
- `io.projparse_merc_spherical_on_ellipsoid` : `+R_C` est ignoré. La chaîne
  sort `+proj=merc +lon_0=2 +k=1 ...` au lieu de `+proj=merc +R_C +lat_0=1 ...`.

### E. Geostationnaire (`geos`)

- `io.projparse_geos_sweep_x` : `+sweep=x` renvoie `unsupported +proj=geos for
  a CRS`.
- `io.projparse_geos_sweep_y` : `+proj=geos +h=1` (balayage par défaut, Y) sort
  `Geostationary Satellite (Sweep X)`. Le nom est écrit en dur dans
  `create_geostationary_satellite_sweep_x` (`src/iso19111/buildop.rs`) et
  l'import n'appelle pas la variante Y.

### F. Mercator de Google et stéréographique

- `io.projparse_merc_google_mercator` : `+a=6378137 +b=6378137 +nadgrids=@null`
  donne un ellipsoïde `unknown` (aplatissement 0) et la méthode `Mercator
  (variant B)`. PROJ écrit `Popular Visualisation Pseudo Mercator`, ID 1024,
  et `DATUM["World Geodetic System 1984"]`.
- `io.projparse_merc_stere` : `+proj=stere +lat_0=30` (non polaire) est nommé
  `Polar Stereographic (variant A)`. PROJ écrit `Stereographic`. C'est une
  erreur de nom côté `src`, pas un écart d'ID.
- `io.projparse_merc_stere_polar_k_and_lat_ts_incompatible` :
  `+lat_0=90 +lat_ts=70 +k=0.994` est accepté. PROJ refuse ce cas avec
  `ParsingException`.

Non tentés, même cause présumée (F) : `io.projparse_merc_google_mercator_non_metre_unit`,
`io.projparse_merc_not_quite_google_mercator`.

### G. Krovak « czech »

- `io.projparse_krovak_czech`, `io.projparse_krovak_modified_czech` : la chaîne
  exportée contient `+axis=swu` (`+proj=krovak +axis=swu +lat_0=49.5 ...`). PROJ
  écrit `+proj=krovak +czech +lat_0=49.5 ...` sans `+axis`. L'export
  (`src/iso19111/projstring.rs`) ajoute `+axis=swu` pour le variant tchèque.

### H. Axe `wsu` du transverse de Mercator

- `io.projparse_tmerc_south_oriented` : `+axis=wsu` est ignoré. Le CRS garde
  `Transverse Mercator` et les axes `(E)`, `(N)`. PROJ écrit `Transverse Mercator
  (South Orientated)` avec `westing` et `southing`. La seconde moitié du test
  (un `+proj=pipeline`) ne passe pas non plus : voir le groupe L.

### L. Pipelines comme CRS (non tentés en `TEST()`, sonde seulement)

- `io.projparse_longlat_complex`, `io.projparse_longlat_unitconvert_deg`,
  `_grad`, `_rad`, `io.projparse_longlat_axisswap` : une sonde a donné
  `unsupported +proj=pipeline for a CRS` pour `+type=crs +proj=pipeline ...`,
  avec `create_crs_from_proj_string` comme avec `create_from_user_input`. Le
  test n'a donc pas été écrit ; la cause est l'import des pipelines.

### M. Non tentés dans ce lot

`io.projparse_utm*`, `laea_*`, `eqc_*`, `ortho_*`, `peirce_*`, `etmerc`,
`tmerc_approx`, `geocent*`, `cart_unit*`, `geoc`, `topocentric`, `ob_tran_*`,
`projected_*`, `*_errors`, `init`, `longlat_title`, `projected_title`,
`non_earth_ellipsoid`, `merc_google_mercator_non_metre_unit` et
`merc_not_quite_google_mercator` (voir F). Ils ne figurent pas dans la liste
de ce lot ; ils restent au ledger.

## 4. Ledger et compteurs

- `docs/UNPORTED-UNIT.md` : 18 lignes retirées de `## test_io.cpp`, de 365 à 347.
  Total des `TEST()` ouverts : 976 → 958.
- `README.md` : `test_io.cpp` 89 → 107 portés, 365 → 347 ouverts ; date des
  compteurs 2026-10-31.
- `ROADMAP.md` : 376 → 394 portés, 976 → 958 ouverts, `test_io` 365 → 347.

## 5. Vérification

Commandes lancées dans le worktree `agent-adf084f5eba1378aa`.

```
cargo test --test test_io                                 # ok, 107 passed, 0 failed (89 avant)
cargo test --all-features --no-fail-fast                  # ok, 606 passed, 0 failed, 27 binaires
cargo clippy --all-targets --all-features -- -D warnings  # ok, aucune sortie
cargo fmt --check                                         # ok
cargo test --test docs_in_sync                            # échec : no test target named `docs_in_sync`
```

`cli_yaml` réécrit `docs/UNPORTED-CLI.md` à chaque suite complète. Ce fichier
est restauré avant le commit et n'en fait pas partie.
