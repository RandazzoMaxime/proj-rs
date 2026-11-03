# Port — lot io-core-b (`test_io.cpp`, classe `core`, 83 derniers noms)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`. Les 83 noms sont les derniers de
`### test_io.cpp (167)` sous `## core` dans `docs/UNIT-TRIAGE.md`, de
`io.projparse_longlat_unitconvert_deg` à `json_import.PointMotionOperation`.
Côté crate : `tests/unit_io_core_b.rs`. Les chaînes et nombres attendus sont
ceux du C++, sans modification.

Le nom Rust d'un `TEST()` est le nom C++ en minuscules, le point remplacé par
`_`. Exemple : `io.EXTENSION_PROJ4` → `io_extension_proj4`.

## 1. Correspondance

| PROJ 9.9.0 | Rust |
|---|---|
| `PROJStringParser` pour un pipeline, `+proj=` isolé, `+geoc`, `+approx`, `+over` | `src/iso19111/projstring.rs` (`pipeline_crs`, `crs_from_proj_string`, `attach_over_extension`) |
| `WKTParser::buildProjectionStandard` : `EXTENSION["PROJ4","+proj=..."]` | `conversion_from_proj4_extension` dans `src/iso19111/wkt_import.rs` |
| `importFromCRSURL` | `import_ogc_crs_url` dans `src/iso19111/json.rs` (avant `AUTH:CODE`) |
| `CRS::setProperties` préfixe `PROJ CRS string: ` | `attach_over_extension` et `Props::identified` |
| `JSONParser` ellipsoid, datum, CRS, conversion, bound, transformation, point motion, concatenated | `src/iso19111/json_read.rs` |
| `JSONFormatter` des mêmes objets | `src/iso19111/json.rs` |
| `POINTMOTIONOPERATION` WKT2:2019 | `wkt_import` pose `point_motion` ; `export_transformation` écrit le mot-clé |

Un point motion n'est pas un type séparé : c'est un `Transformation` avec
`point_motion`. L'export PROJJSON écrit `PointMotionOperation`, le WKT écrit
`POINTMOTIONOPERATION`. Le test C++ ne distingue que ces textes.

`fixSteps` n'est pas appliqué à l'import d'un `ConcatenatedOperation`. Le
JSON du test PROJ 9.9.0 revient identique, donc `fixSteps` n'y change rien.

## 2. Portés (80)

Les 80 `TEST()` ont un test du même nom dans `tests/unit_io_core_b.rs` et leur
ligne est retirée de `docs/UNPORTED-UNIT.md`. Les trois noms de la section 3
restent au ledger.

## 3. Ouverts (3)

- `io.projparse_init` : le mode de compatibilité PROJ4 (`init=epsg:4326` en
  longitude/latitude, `init=epsg:3040` en easting/northing, `+title=`,
  `+over`, un pipeline qui conserve `init=`) et l'opération
  `init=ITRF2000:ITRF2005` ne sont pas le parseur CRS de
  `src/iso19111/projstring.rs`. Ce parseur lit `init=` comme un paramètre.
- `io.createFromUserInput` : `create_from_user_input` ne renvoie qu'un `Crs`.
  Le test C++ exige aussi un datum, un ellipsoïde, un méridien, un ensemble,
  une opération, des URN `urn:ogc:def:*` et un `CoordinateMetadata`
  (`ITRF2014@2025.1`). Élargir le type de retour casserait les appelants.
- `json_import.bound_crs_with_source_crs_in_transformation` : la source est un
  `DerivedGeographicCRS`. Ce genre de CRS n'existe pas dans `Crs`.

## 4. Quirks conservés

- Zone UTM sud : un `Conversion` nommé `UTM zone 55S` avec le code 17055
  devient 16155 (`build_id`).
- `EPSG:102100` est réessayé comme ESRI.
- `+proj=longlat +geoc` est un CRS sphérique planétocentrique ; l'export
  PROJ échoue, comme le `throw` du test.
- `+proj=mbt_s` sans méthode connue : les drapeaux inconnus sont retirés.
  Le WKT1 `PROJECTION["custom_proj4"]` avec `EXTENSION["PROJ4",...]` reconstruit
  `METHOD["PROJ mbt_s"]` et ses paramètres. `Mercator_1SP` et
  `Polar_Stereographic` gardent leur lecture WKT1.
- `+over` n'entre pas dans la conversion. Il reste dans `extension_proj4` et
  dans la remarque `PROJ CRS string: `.

## 5. Vérification

```
cargo test --offline --test unit_io_core_b
cargo clippy --all-targets --all-features --offline -- -D warnings
cargo test --offline --test docs_in_sync
```

`unit_io_core_b` : 80 passés. `docs_in_sync` peut échouer sur les comptes
générés de `README.md`, `ROADMAP.md` et `docs/UNIT-TRIAGE.md` : ces fichiers
ne sont pas édités ici.
