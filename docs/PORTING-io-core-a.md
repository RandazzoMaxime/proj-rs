# Port — lot `io-core-a` (`test_io.cpp`, classe `core`, 84 premiers noms)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, les 84 premiers `TEST()` de
`docs/UNIT-TRIAGE.md` (`### test_io.cpp` sous `## core`), de
`wkt_parse.wkt1_geographic_old_datum_name_without_EPSG_code` à
`io.projparse_longlat_axis_swu`. Les noms suivants de la section ne sont pas
de ce lot.

Les tests Rust sont dans `tests/unit_io_core_a.rs`. Le nom de fonction est le
nom C++ en snake_case (`wkt_parse.COORDINATEOPERATION` →
`wkt_parse_coordinateoperation`).

## Correspondance

57 noms passent. Chacun a un `#[test]` du même nom dans
`tests/unit_io_core_a.rs`. Les groupes :

| PROJ 9.9.0 | Comportement porté |
|---|---|
| `wkt_parse.wkt1_geographic_*`, `wkt1_geocentric_with_PROJ4_extension`, `wkt1_projected_with_PROJ4_extension` | WKT1, `EXTENSION["PROJ4"]`, alias de datum (y compris le nom tchèque accentué) |
| `wkt_parse.wkt2_spherical_planetocentric` | `GEODCRS` sphérique, `ID` versionné IAU sans base |
| `wkt_parse.wkt1_krovak_south_west`, `wkt2_krovak_modified_*` | Krovak / Krovak modifié, `+axis=swu`, pipeline `axisswap` |
| `proj_export.wkt2_hotine_oblique_mercator_without_rectified_grid_angle` | Hotine A (EPSG:9812) sans gamma fictif : `+proj=omerc +no_uoff` |
| `wkt_parse.wkt2_2019_projected_*`, `wkt2_2019_eqdc_non_epsg`, `crs.projected_angular_unit_from_primem` | WKT2:2019, base géocentrique, unité angulaire du méridien |
| `wkt_parse.VERTCS_*`, `vertcrs_WKT1_LAS_*`, `compoundcrs_WKT1_LAS_only_geoid_name`, `vdatum_with_ANCHOR*` | VERTCS ESRI, LAS, ancre de datum vertical |
| `wkt_parse.COMPD_CS_*`, `implicit_compound_CRS_*_ESRI` | `COMPD_CS` laxiste (horizontal+horizontal, hauteur ellipsoïdale), export ESRI 3D |
| `wkt_parse.COORDINATEOPERATION*` | Opération, `INTERPOLATIONCRS`, paramètre EPSG:1048 résolu avec `proj.db`, `VERSION` seulement en WKT2:2019 |
| `wkt_parse.conversion_proj_based`, `conversion_utm_zone_south_wrong_id` | Méthode PROJ, code UTM sud corrigé à l'import |
| `wkt_parse.BOUNDCRS_*` | Helmert abrégé, signe des rotations, CRS d'interpolation |
| `wkt_parse.geogcs_TOWGS84_3terms`, `projcs_TOWGS84_7terms`, `io.projcs_TOWGS84_7terms_autocorrect` | `TOWGS84` → `BoundCRS` vers EPSG:4326 ; autocorrection du signe seulement avec une base |
| `wkt_parse.WKT1_*_EXTENSION` | `EXTENSION["PROJ4_GRIDS"]` horizontal (NTv2) et vertical (géoïde), réexport WKT1 du `COMPD_CS` |
| `wkt_parse.EDATUM`, `ENGCRS*`, `PDATUM`, `ensemble*` | Datum d'ingénierie, paramétrique, ensemble |
| `wkt_parse.esri_geogcs_datum_spheroid_name_from_db_substitution`, `esri_datum_name_with_prime_meridian`, `wkt1_esri_case_insensitive_names`, `wkt1_esri_non_expected_param_name`, `wkt1_esri_normalize_unit` | Alias ESRI et unité Gold Coast |
| `wkt_parse.invalid_CoordinateMetadata` | `COORDINATEMETADATA` sans CRS ou époque invalide est refusé |

## API

Pas de nouvelle surface publique hors des accesseurs déjà présents sur les
autres opérations : `Transformation::version`. L'import WKT et l'export WKT1
restent sur `WktParser` et `WktFormatter`.

| PROJ 9.9.0 | PROJ-RS |
|---|---|
| `WKTParser::createFromWKT` | `WktParser::create_from_wkt` |
| `WKTFormatter::create` / `setOutputId(false)` | `WktFormatter::create` / `hide_identifiers` |
| `BoundCRS::createFromTOWGS84` | `WktParser::bind_towgs84` (import WKT) et `bound_import::bind_towgs84` (chaîne PROJ) |
| `BoundCRS::createFromNadgrids` | `bound_import::bind_nadgrids` |
| `CompoundCRS::createLax` pour `COMPD_CS` | `lax_compd_cs` dans `wkt_import.rs` |
| `SingleOperation::exportTransformationToWKT` (`INTERPOLATIONCRS`) | `export_transformation` |
| `DatabaseContext` autocorrection `TOWGS84` | `DatabaseContext::to_wgs84_autocorrect_wrong_values` |
| `exportAsESRIWktCompoundCRSWithEllipsoidalHeight` | `export_esri_projected_3d` / `esri_vertical_code_for_geodetic` |

## Écarts

Ces 27 noms restent dans `docs/UNPORTED-UNIT.md`.

| Nom | Raison |
|---|---|
| `wkt_parse.CONCATENATEDOPERATION` | L'export WKT d'une opération concaténée est refusé (`export_operation`). Le test construit l'objet, l'exporte, puis le relit. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_and_conversion` | Même export WKT, égalité stricte du WKT2:2019, et pipeline `+inv +proj=utm`. Un pas `CONVERSION` n'a pas de chaîne PROJ tant que la paire de CRS du pas n'est pas déduite du `SOURCECRS` / `TARGETCRS`. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_coordinateoperation_conversion` | Idem, avec un pas `hgridshift` (`CTABLE2`, grille `conus`) entre deux UTM. |
| `wkt_parse.CONCATENATEDOPERATION_with_conversion_coordinateoperation_to_inverse_conversion` | Le pipeline inverse le `hgridshift` (`+inv`) parce que le dernier pas est l'inverse de la conversion d'arrivée. |
| `wkt_parse.CONCATENATEDOPERATION_with_inverse_conversion_of_compound` | Le pas « PROJ-based » (pipeline stéréo / `hgridshift` / `vgridshift`) est aplati et les `axisswap` / `unitconvert` de tête sont retirés. |
| `wkt_parse.esri_projcs` | Table `esriProjDefs` : chaque nom de projection ESRI, ses paramètres WKT2, et le réexport WKT1:ESRI. |
| `wkt_parse.wkt1_esri_krovak_south_west` | Orthographe ESRI de Krovak et axes sud-ouest, au-delà du chemin WKT2 déjà porté. |
| `wkt_parse.wkt1_esri_krovak_east_north_non_standard_likely_from_GDAL_wkt1` | Variante GDAL est-nord non standard. |
| `wkt_parse.wkt1_esri_ups_north`, `wkt1_esri_ups_south`, `wkt1_esri_wgs_1984_ups_north_E_N`, `wkt1_esri_wgs_1984_ups_south_E_N` | UPS ESRI (noms, hémisphère, ordre d'axes). |
| `wkt_parse.wkt1_esri_gauss_kruger` | Projection Gauss-Kruger ESRI. |
| `wkt_parse.wkt1_esri_goode_homolosine_without_option_0` | Goode Homolosine sans paramètre d'option. |
| `wkt_parse.wkt1_oracle`, `wkt1_oracle_albers_conical_equal_area` | Dialecte Oracle. |
| `wkt_parse.wkt1_lcc_1sp_without_1sp_suffix` | LCC 1SP reconnu sans le suffixe du nom de méthode. |
| `wkt_parse.wkt1_pseudo_wkt1_gdal_pseudo_wkt1_esri` | Pseudo-Mercator : les trois dialectes WKT1 doivent réexporter la même chaîne. |
| `wkt_parse.invalid_EngineeingCRS` | `build_cs` refuse `CS[temporal,1]`. Pas de système de coordonnées temporel. Le test est `#[ignore]`. |
| `io.projstringformatter_optim_hgridshift_vgridshift_hgridshift_inv` | Optimisation de pipeline sur une chaîne grille horizontale / verticale / horizontale inverse. |
| `io.projstringformatter_optim_as_uc_vgridshift_uc_as_push_as_uc` | Même optimiseur : `axisswap`, `unitconvert`, `vgridshift`, `push`. |
| `io.projstringformatter_krovak_to_krovak_east_north`, `io.projstringformatter_krovak_east_north_to_krovak` | Réduction d'un aller-retour Krovak / Krovak est-nord à un `axisswap`. |
| `io.projparse_longlat_complex` | `createOperation` de EPSG:4326 vers le CRS parsé (Paris, grades, `axisswap`). `create_operation_proj_string` ne couvre qu'une cible projetée ou géocentrique. |
| `io.projparse_longlat_axis_enu` | Axes `enu` puis opération `+proj=axisswap +order=2,1`. Même limite de l'usine d'opérations. |
| `io.projparse_longlat_axis_neu` | Axes `neu` puis opération `+proj=noop`. |
| `io.projparse_longlat_axis_swu` | Axes `swu` puis pipeline de deux `axisswap`. |

## Comportements conservés

- Un `COMPD_CS` horizontal+horizontal équivalent promeut le premier CRS et
  réexporte deux copies 2D. Le nom est `nom + nom` (géographique) ou
  `projection + base` (projeté). On ne substitue pas EPSG:4979.
- `TOWGS84` à 3 termes n'est pas passé à l'autocorrection SQL : les rotations
  complétées par des zéros sortent tout de suite. L'autocorrection à 7 termes
  n'a lieu que si une base est attachée au parseur.
- `EXTENSION["PROJ4_GRIDS"]` l'emporte sur `TOWGS84` du même datum : les grilles
  sont effacées quand `TOWGS84` est lié, comme dans PROJ.
- Les deux listes de grilles géoïde que GDAL écrivait pour EPSG:5703 ne
  deviennent pas un `BoundCRS`.
- Hotine variante A sans « Angle from Rectified to Skew Grid » n'ajoute pas de
  `+gamma`.
- Une fois qu'un nœud WKT2 porte un `ID`, les nœuds intérieurs omettent le
  leur, sauf `METHOD` et `PARAMETER`.

## Vérification

```
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
export SDKROOT=/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX26.2.sdk
cargo test --offline --test unit_io_core_a
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline --test docs_in_sync
```

`unit_io_core_a` : 57 passés, 1 ignoré (`invalid_EngineeingCRS`). Clippy est
propre. `docs_in_sync` échoue sur les comptes générés de `README.md` et
`docs/UNIT-TRIAGE.md` ; ces fichiers ne sont pas édités ici.
