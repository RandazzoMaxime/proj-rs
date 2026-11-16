# PORTING: le reste du corpus d'exports

Lot : vider `tests/parity/export/known.tsv` (1102 lignes avant, 0 après). Chaque cause a son test
de caractérisation, dont les valeurs viennent de `projinfo` 9.9.0, et son commit. Le corpus est
décrit dans `docs/PARITY.md`, « Exports d'objets ».

## Causes

| Cause | Code de PROJ | Port | Test | Lignes |
|---|---|---|---|---|
| Géographique/géocentrique : `Conversion`, pas `Transformation` | `concatenatedoperation.cpp:571-645`, `conversion.cpp:2611`, `2742`, `coordinateoperationfactory.cpp:5311-5353` | `factory.rs` (`geographic_geocentric`), `geodetic_ops.rs` (`geographic_geocentric_conversion`), `buildop.rs` (`Conversion::inverse`), `conversion_export.rs` | `export_geocentric_bridge_conversion` | 633 |
| Conversion sans paire de CRS : l'étape seule | `conversion.cpp:3768-4340` | `conversion_export.rs` (`export_steps`) | `operation_proj_export_projection` | 97 |
| Unité sans nom PROJ : `+to_meter` | `crs.cpp:4758` | `projstring.rs` (`linear_unit_clause`) | `crs_proj_export_units_datum` | 9 |
| `+R_A` et `+f=0` absents sur une base sphérique | `conversion.cpp:4316-4333` | `projstring.rs` (`ProjectedCRS::export_with`) | idem | 3 |
| Géocentrique sur un ensemble : `datumNonNull` | `crs.cpp:631-662`, `2544` | `projstring.rs`, `bound.rs`, `equiv.rs` | idem | 1 |
| Grille du registre gardée hors substitution | `singleoperation.cpp:2402` (`return self`) | `factory.rs` (`with_proj_grid_names`) | `export_grid_without_substitution` | 3 |
| Messages d'erreur WKT d'une opération | `singleoperation.cpp:3045`, `5712` | `wkt.rs` (`export_transformation`) | `export_operation_wkt_errors` | 107 |
| `LOCAL_DATUM` sans `AUTHORITY` | `datum.cpp:2739-2755` | `wkt.rs` (`export_engineering_datum`) | `export_wkt1_engineering` | 128 |
| Noms WKT1 des paramètres | `parammappings.cpp:1882`, `singleoperation.cpp:930-949`, `conversion.cpp:3304` | `wkt.rs` (`lookup_method`, `wkt1_parameter_name`) | `export_wkt1_parameter_names` | 113 |
| Alias ESRI d'un datum par nom seul | `datum.cpp:1373-1447` | `wkt.rs` (`wkt1_datum_name`) | `export_wkt1_esri_datum_alias` | 4 |
| Base 2D d'un CRS projeté 3D en ESRI | `crs.cpp:2073` | `wkt.rs` (`export_esri_projected_3d`) | `export_wkt1_esri_projected_3d` | 1 |
| Définition ESRI enregistrée d'un CRS EPSG | `crs.cpp:4397-4457`, `4687-4696` | `wkt.rs` (`esri_projected_text`), `factory.rs`, `equiv.rs` | `export_wkt1_esri_registered_text` | 3 |

## Ce que PROJ fait et que le port faisait autrement

- Dans PROJ, tout pas géographique/géocentrique d'une opération est une `Conversion`
  (`createGeographicGeocentric`). Elle ne compte pas dans `getTransformationStepCount`
  (`coordinateoperationfactory.cpp:1195`) : le port ne la comptait pas non plus tant que seuls les
  pas de `fixSteps` étaient des conversions, ce qui retirait un candidat du pivot
  (`tests/pivot_candidate_set.rs`). La fabrique de `createOperationsGeodToGeod` donne maintenant
  des `Conversion` aussi. Seul le pas forcé en ballpark (`createGeodToGeodPROJBased`,
  `forced_geographic_geocentric`) reste une `Transformation`.
- `getMapping(const OperationMethod *)` ne parcourt que les mappings de projection. Une méthode
  qui n'y est pas (Lambert Conic Near-Conformal, Polar Stereographic variante C) écrit les noms EPSG
  de ses paramètres, comme le dialecte ESRI, qui ne passe aucun mapping. Le port inventait des
  noms (`latitude_of_origin`, `False_Easting`).
- Le dialecte WKT1_ESRI écrit le texte de `text_definition` du CRS ESRI de même nom quand les deux
  CRS sont équivalents. L'équivalence relâche l'ordre des axes de la base quand un des CS est
  implicite (un `GEOGCS` ESRI n'a pas d'`AXIS`).

## Écarts volontaires

Aucun. Le registre est vide.

## Tests

- Les fichiers de `tests/` cités ci-dessus.
- `cargo test --test parity parity_export -- --ignored` : 8 935 invocations, 311 242 lignes, toutes
  identiques.
- `cargo test --test parity` et `parity_wide` : inchangés (voir `docs/PARITY.md`).
