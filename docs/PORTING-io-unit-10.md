# Port — `wkt_parse.wkt1_*` : import WKT1 (lot io-10, lot 6)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, groupe `wkt_parse.wkt1_*`
(chaînes WKT1 copiées octet pour octet dans le fichier de test). Côté crate :
`tests/test_io_wkt1.rs` (nouveau, 20 `TEST()` candidats), `src/iso19111/wkt.rs`,
`src/iso19111/wkt_import.rs`.

Ce lot porte **16 `TEST()`** sur les 20 candidats. Il laisse **4 tests ouverts**,
avec la cause de chacun. Le lot 5 (PROJJSON) n'est pas porté (voir la dernière
section).

## 1. Portés (16)

| PROJ 9.9.0 `wkt_parse.*` | PROJ-RS `tests/test_io_wkt1.rs` |
|---|---|
| `wkt1_esri_EPSG_4901_grad` | `wkt_parse_wkt1_esri_epsg_4901_grad` |
| `wkt1_esri_LINUNIT` | `wkt_parse_wkt1_esri_linunit` |
| `wkt1_geographic_old_datum_name_from_EPSG_code` | `wkt_parse_wkt1_geographic_old_datum_name_from_epsg_code` |
| `wkt1_geographic_deprecated` | `wkt_parse_wkt1_geographic_deprecated` |
| `wkt1_non_conformant_inf_inverse_flattening` | `wkt_parse_wkt1_non_conformant_inf_inverse_flattening` |
| `wkt1_esri_GCS_unknown_D_unknown` | `wkt_parse_wkt1_esri_gcs_unknown_d_unknown` |
| `wkt1_esri_GCS_unknown_D_Unknown_based_on_WGS_84_ellipsoid` | `wkt_parse_wkt1_esri_gcs_unknown_d_unknown_based_on_wgs_84_ellipsoid` |
| `wkt1_projected` | `wkt_parse_wkt1_projected` |
| `wkt1_projected_no_axis` | `wkt_parse_wkt1_projected_no_axis` |
| `wkt1_projected_missing_unit_and_axis` | `wkt_parse_wkt1_projected_missing_unit_and_axis` |
| `wkt1_projected_wrong_axis_geogcs` | `wkt_parse_wkt1_projected_wrong_axis_geogcs` |
| `wkt1_projected_wrong_angular_unit` | `wkt_parse_wkt1_projected_wrong_angular_unit` |
| `wkt1_hotine_oblique_mercator_without_rectified_grid_angle` | `wkt_parse_wkt1_hotine_oblique_mercator_without_rectified_grid_angle` |
| `wkt1_hotine_oblique_mercator_with_rectified_grid_angle` | `wkt_parse_wkt1_hotine_oblique_mercator_with_rectified_grid_angle` |
| `wkt1_hotine_oblique_mercator_azimuth_center_with_rectified_grid_angle` | `wkt_parse_wkt1_hotine_oblique_mercator_azimuth_center_with_rectified_grid_angle` |
| `wkt1_krovak_north_oriented` | `wkt_parse_wkt1_krovak_north_oriented` |

Les 16 lignes correspondantes sont retirées de `docs/UNPORTED-UNIT.md`.

## 2. Ouverts (4), avec la cause

- **`wkt1_geographic_with_PROJ4_extension`**, **`wkt1_geocentric_with_PROJ4_extension`**,
  **`wkt1_projected_with_PROJ4_extension`** : l'extension `EXTENSION["PROJ4", ...]`
  est perdue à l'import comme à l'export (aucun nœud n'est conservé, et la
  chaîne PROJ produite n'a pas `+wktext`). Il faut un champ dans le modèle
  pour porter cette extension. Ce n'est pas une correction locale.
- **`wkt1_geographic_old_datum_name_without_EPSG_code`** : le datum
  `System_Jednotne_Trigonometricke_Site_Katastralni_Ferro` n'a pas de ligne
  d'alias dans `data/proj.db` (seule l'orthographe ESRI
  `System_of_the_Unified_..._(Ferro)` et les noms tchèques existent), et il n'a
  pas de code `AUTHORITY`. Le nom n'est donc pas relié. La source de correspondance
  utilisée par PROJ n'est pas reproduite.

Non tenté dans ce lot :

- **`wkt1_krovak_south_west`** : sa seconde moitié importe un CRS
  `+proj=pipeline`, que le crate refuse (`unsupported +proj=pipeline for a CRS`).
- Les cas `esri_*` et `wkt1_esri_*` plus bas dans le registre, et
  `proj_export.wkt2_hotine_*` (ce n'est pas `wkt_parse`).

## 3. Changements de source

Tous sont gardés. Aucun n'a été annulé.

- **(a)** `src/iso19111/wkt.rs`, `export_conversion_wkt1` : la variante Hotine
  WKT1 (non ESRI), méthodes 9812/9815 sans `rectified_grid_angle`, reçoit
  `PARAMETER["rectified_grid_angle", azimuth]`, comme le fait PROJ.
- **(b)** `src/iso19111/wkt_import.rs`, `reconcile_geographic_id` : un GEOGCS de
  base sans `AXIS`, de même unité que l'autorité, prend le système de
  coordonnées de l'autorité (latitude d'abord). Auparavant ce n'était le cas que
  sans `UNIT`. **Vérifié** : la suite complète ne montre aucune régression
  (voir section 5).
- **(c)** `wkt1_method` : `Hotine_Oblique_Mercator` simple se mappe sur la
  variante A, 9812, comme PROJ.
- **(d)** `wkt1_utm_named` (nouveau) : un Transverse Mercator WKT1 identique à
  `Conversion::utm_zone()` devient `Conversion::create_utm_north/south`
  (« UTM zone NN[N|S] »). Chemin WKT1 uniquement.
- **(e)** `build_wkt1_projection` : Krovak avec `AXIS EAST` puis `NORTH` devient
  `Krovak (North Orientated)` (1041), via `axes_are_east_north`. Pour que le
  test passe, les paramètres du 1041 sont renommés avant le mappage WKT1 générique
  (`latitude_of_center` → 8811 « Latitude of projection centre », `longitude_of_center`
  → 8833, `azimuth` → 1036, `pseudo_standard_parallel_1` → 8818, `scale_factor` → 8819).
  Sans ce renommage, le générique attribuait 8801/8802/8813/8805, ce qui n'est
  pas le jeu de paramètres du 1041. **Cause trouvée et corrigée.**
- **(f)** `use super::buildop::{OpProps, UtmHemisphere};` ajouté dans
  `wkt_import.rs`.

## 4. Lot 5 not done: what it needs

Le lot 5 (PROJJSON, `json_import`) est abandonné. Il faut une API publique nouvelle,
qui n'a pas été ajoutée. Analyse, à reprendre pour le prochain lot :

- `src/iso19111/json.rs` n'importe que `GeographicCRS`. Tout autre `"type"` est
  refusé (« PROJJSON type X is not imported in this build »).
- `Crs::export_to_json` n'écrit que `GeographicCRS` et code en dur l'URL du
  schéma. Les tests PROJ appellent `setSchema("foo")` et comparent le texte
  entier. Il faut donc un paramètre `schema` à l'export.
- Les objets autonomes `Ellipsoid`, `Axis`, `PrimeMeridian` et
  `GeodeticReferenceFrame` n'ont ni import ni export public. `createFromUserInput`
  renvoie un `Crs`. Les tests ellipsoïde, méridien et axe ne peuvent donc pas
  passer. Leurs lecteurs (`read_ellipsoid`, `read_meridian`) sont privés.
- Les tests de CRS géographiques demandent aussi l'export de `remarks`, et la
  plupart des autres demandent l'export Projected, Geodetic, Compound et Bound.
- Les tests d'erreur seuls (`ellipsoid_errors`, `axis_with_invalid_*`) échoueraient
  pour la mauvaise raison (type non supporté au lieu de `ParsingException`). Ils
  ne doivent pas compter comme réussis.

## 5. Vérifications

Codes de sortie réels, à compléter après les vérifications finales (section 6).

- `cargo test --all-features --test test_io_wkt1` : 16 réussis, 4 échoués
  (les quatre tests ouverts de la section 2).
- `cargo test --all-features --no-fail-fast` : 719 réussis, 4 échoués. Les 4
  échecs sont les quatre tests ouverts. Aucune régression, y compris pour (b).
