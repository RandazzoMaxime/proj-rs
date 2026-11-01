# Port — `wkt_parse.*` : WKT2, CRS verticaux, composés et liés (lot io-11)

Source : PROJ 9.9.0, `test/unit/test_io.cpp`, groupe `wkt_parse.*` de la classe
`core` de `docs/UNIT-TRIAGE.md` : `wkt2_*`, `vertcrs_*`, `VERTCS_*`,
`COMPOUNDCRS` / `COMPD_CS_*`, `implicit_compound_CRS_*`, `BOUNDCRS_*` et
`boundcrs_of_verticalcrs_*`. Côté crate : `tests/test_io_wkt2.rs` (nouveau,
41 `TEST()` candidats), `src/iso19111/wkt_import.rs` (deux retouches locales).

Ce lot porte **20 `TEST()`** et laisse **21 tests ouverts**, avec la cause de
chacun. Les chaînes et nombres attendus sont ceux de PROJ, sans modification.

## 0. Provenance du source C++

Le téléchargement direct de `test_io.cpp` (dépôt OSGeo, tag 9.9.0) a été refusé
par le classifieur d'autorisation de la session. Le fichier utilisé est la copie
déjà présente dans le dossier de travail parent (`scratchpad/test_io.cpp`,
18 856 lignes), recopiée dans `scratchpad/d/test_io.cpp`. Sa version 9.9.0 n'a
pas été vérifiée octet pour octet. Contrôle fait : les noms `wkt_parse.*` du
ledger qui concernent ce lot existent tous dans la copie (aucun manquant).

## 1. Portés (20)

| PROJ 9.9.0 `wkt_parse.*` | PROJ-RS `tests/test_io_wkt2.rs` |
|---|---|
| `wkt2_epsg_org_EPSG_4901_PRIMEM_weird_sexagesimal_DMS` | `wkt_parse_wkt2_epsg_org_epsg_4901_primem_weird_sexagesimal_dms` |
| `wkt2_simplified_EPSG_4326` | `wkt_parse_wkt2_simplified_epsg_4326` |
| `wkt2_EPSG_4979` | `wkt_parse_wkt2_epsg_4979` |
| `wkt2_geocentric_DEFININGTRANSFORMATION` | `wkt_parse_wkt2_geocentric_definingtransformation` |
| `wkt2_projected` | `wkt_parse_wkt2_projected` |
| `wkt2_2019_projected_with_id_in_basegeodcrs` | `wkt_parse_wkt2_2019_projected_with_id_in_basegeodcrs` |
| `wkt2_2019_projected_no_id_but_id_in_basegeodcrs` | `wkt_parse_wkt2_2019_projected_no_id_but_id_in_basegeodcrs` |
| `wkt2_2019_simplified_projected` | `wkt_parse_wkt2_2019_simplified_projected` |
| `vertcrs_VRF_WKT2` | `wkt_parse_vertcrs_vrf_wkt2` |
| `vertcrs_with_GEOIDMODEL` | `wkt_parse_vertcrs_with_geoidmodel` |
| `vertcrs_WKT1_GDAL` | `wkt_parse_vertcrs_wkt1_gdal` |
| `vertcrs_WKT1_GDAL_minimum` | `wkt_parse_vertcrs_wkt1_gdal_minimum` |
| `vertcrs_WKT1_GDAL_missing_unit_and_axis` | `wkt_parse_vertcrs_wkt1_gdal_missing_unit_and_axis` |
| `vertcrs_WKT1_GDAl_missing_unit_with_axis` | `wkt_parse_vertcrs_wkt1_gdal_missing_unit_with_axis` |
| `VERTCS_WKT1_ESRI` | `wkt_parse_vertcs_wkt1_esri` |
| `VERTCS_WKT1_ESRI_down` | `wkt_parse_vertcs_wkt1_esri_down` |
| `vertcrs_with_ensemble` | `wkt_parse_vertcrs_with_ensemble` |
| `COMPOUNDCRS` | `wkt_parse_compoundcrs` |
| `boundcrs_of_verticalcrs_to_geog3Dcrs` | `wkt_parse_boundcrs_of_verticalcrs_to_geog3dcrs` |
| `implicit_compound_CRS_ESRI` | `wkt_parse_implicit_compound_crs_esri` |

Les 20 lignes correspondantes sont retirées de `docs/UNPORTED-UNIT.md`.

## 2. Ouverts (21), avec la cause

Chaque test ouvert porte `#[ignore = "open: ..."]` et reste dans le ledger. Son
corps est celui de PROJ : aucune valeur attendue n'a été modifiée.

### 2.1 COMPD_CS plus élévation ellipsoïdale : `Compound` au lieu de CRS 3D ou lié (6 tests)

Dans `src/iso19111/wkt_import.rs`, le constructeur de composé garde un
`CompoundCRS` pour `COMPD_CS` (horizontal + `VERT_CS` « Ellipsoid »). PROJ
replie ce cas : un `GeographicCRS` ou `ProjectedCRS` 3D, ou un `BoundCRS` quand
le datum porte `TOWGS84`.

- `wkt_parse.COMPD_CS_non_conformant_horizontal_plus_horizontal_as_in_LAS` : PROJ donne un `ProjectedCRS` à 3 axes.
- `wkt_parse.COMPD_CS_non_conformant_horizontal_TOWGS84_plus_horizontal_as_in_LAS` : PROJ donne un `BoundCRS`.
- `wkt_parse.COMPD_CS_horizontal_bound_geog_plus_vertical_ellipsoidal_height` : PROJ donne un `BoundCRS`.
- `wkt_parse.COMPD_CS_horizontal_projected_plus_vertical_ellipsoidal_height` : PROJ donne un `ProjectedCRS` à 3 axes.
- `wkt_parse.COMPD_CS_horizontal_geog_plus_vertical_ellipsoidal_height_non_metre` : PROJ donne un `GeographicCRS` à 3 axes, nommé « NAD83 (Ellipsoid (US Feet)) ».
- `wkt_parse.implicit_compound_CRS_projected_with_ellipsoidal_height_ESRI` : même repli, `ProjectedCRS` à 3 axes attendu.

Vérifié par une sonde temporaire (supprimée) : les sept imports renvoient
`Compound` à 2 composants.

### 2.2 Noms verticaux LAS (3 tests)

`NAVD88 - Geoid03 (Feet)`, `NAVD88 via Geoid09` et `Geoid 2012A` gardent leur
nom WKT. PROJ les renomme `NAVD88 height (ftUS)` / `NAVD88 height`, avec
l'identifiant EPSG 6360 / 5703. Aucune ligne d'alias de `proj.db` ne couvre ces
noms (vérifié) : la règle LAS de PROJ n'existe pas dans ce crate.

- `wkt_parse.vertcrs_WKT1_LAS_ftUS`
- `wkt_parse.vertcrs_WKT1_LAS_metre`
- `wkt_parse.compoundcrs_WKT1_LAS_only_geoid_name`

### 2.3 Alias ESRI `EGM2008_Geoid` (1 test)

- `wkt_parse.VERTCS_WKT1_ESRI_context` : la ligne d'alias `EGM2008_Geoid` →
  EPSG 3855 a pour source `ESRI_OLD` dans `proj.db`. La recherche de
  `src/iso19111/wkt_import.rs` ne demande que la source `ESRI`, donc le nom
  reste `EGM2008_Geoid`.

### 2.4 Paramètres sans identifiant (2 tests)

Pour la méthode `Transverse Mercator (3D)`, `Longitude of origin` n'a pas
d'identifiant : `lon_0` sort à 0. Dans le cas UTM, la zone n'est pas reconnue
et la sortie est `+proj=tmerc`. Table : `src/iso19111/parammap.rs` (méthode 9807
3D), lecture dans `src/iso19111/wkt_import.rs`.

- `wkt_parse.wkt2_2019_projected_3D`
- `wkt_parse.wkt2_2019_projected_utm_3D`

### 2.5 Equidistant Conic (1 test)

- `wkt_parse.wkt2_2019_eqdc_non_epsg` : `lat_0`, `lon_0`, `x_0` et `y_0` sortent à 0. PROJ lit `Latitude / Longitude of natural origin` et `False easting` / `False northing` (codes 8801, 8802, 8806, 8807) pour cette méthode. Table : `src/iso19111/parammap.rs`, ligne « Equidistant Conic ».

### 2.6 Chaînes `+proj=pipeline` (2 tests)

- `wkt_parse.wkt2_krovak_modified_south_west` et `wkt_parse.wkt2_krovak_modified_east_north` : la première partie passe (export WKT2 et chaîne PROJ `+proj=mod_krovak`). L'import de la chaîne `+proj=pipeline ...` est refusé : `unsupported +proj=pipeline for a CRS` (`src/iso19111/projstring.rs`). Même cause que `wkt1_krovak_south_west` dans `docs/PORTING-io-unit-10.md`.

### 2.7 Identifiant `ID["IAU",19902,2015]` (1 test)

- `wkt_parse.wkt2_spherical_planetocentric` : la version est repliée dans le nom d'autorité à l'import : l'export donne `ID["IAU_2015",19902]`. Lecture et écriture de l'identifiant : `src/iso19111/wkt_import.rs` et `src/iso19111/wkt.rs`. Le modèle `Identifier` n'a pas de champ de version : changement de modèle, non fait ici.

### 2.8 Base géocentrique (1 test)

- `wkt_parse.wkt2_2019_projected_with_base_geocentric` : la base `BASEGEODCRS` avec `ID["EPSG",4978]` et un `ENSEMBLE` sort géographique. PROJ la garde géocentrique. Constructeur de base dans `src/iso19111/wkt_import.rs`.

### 2.9 Nom ESRI de la VERTCS implicite (1 test)

- `wkt_parse.implicit_compound_CRS_geographic_with_ellipsoidal_height_ESRI` : l'export ESRI nomme la VERTCS d'après le datum (`D_WGS_1984`, `src/iso19111/wkt.rs`, `wkt1_datum_name`). PROJ écrit `WGS_1984`, le nom d'entrée.

### 2.10 Paramètre d'interpolation (1 test)

- `wkt_parse.BOUNDCRS_with_interpolation_as_parameter` : le paramètre `EPSG code for Interpolation CRS` n'est lu nulle part dans `src/iso19111/wkt_import.rs` ni `src/iso19111/bound_import.rs`. Sans base, il reste un paramètre au lieu de deux.

### 2.11 Lié à TOWGS84 (2 tests, marqués `ignore`)

- `wkt_parse.BOUNDCRS_transformation_from_names` et `wkt_parse.BOUNDCRS_transformation_from_codes` : les sept valeurs `getTOWGS84Parameters(true)` ne sont pas accessibles. `Transformation` n'a pas d'accesseur public (`towgs84_lenient` est `pub(crate)`). Les noms source et cible sont vérifiés dans le test ; la partie TOWGS84 reste ouverte.

## 3. Hors lot (non portés)

- Les noms `wkt_parse.COMPOUNDCRS_spatio_*`, `dynamic_vertical_reference_frame`
  et `VERTCS_with_ellipsoidal_height_ESRI` ne sont pas dans la classe `core`
  de `docs/UNIT-TRIAGE.md` : ils relèvent du lot `niche` (temporel, paramétrique,
  dynamique).
- Aucun nom `epsg_org_api` n'est dans ce lot.

## 4. Changements de source

Deux retouches, toutes deux dans `src/iso19111/wkt_import.rs`. Aucune autre
fonction ni fichier de src n'a été touché.

1. **`build_axis`** (`buildCS: missing UNIT`) : un `AXIS` linéaire sans `UNIT`
   est refusé en mode strict (inchangé) ; en mode lenient il est lu en mètres
   avec un avertissement (`self.recover`), comme PROJ. Motif : le test
   `vertcrs_WKT1_GDAl_missing_unit_with_axis` demande cette lecture, et
   `PROJ` le fait. Retouche de quatre lignes.
2. **`build_vertical`** : en mode ESRI (`esri || maybe_esri`), le nom de la
   CRS verticale est remplacé par son nom officiel trouvé dans la table
   `vertical_crs` de `proj.db`, comme le font déjà les CRS projetées. Motif :
   `implicit_compound_CRS_ESRI` (`CGVD2013_height` → « CGVD2013(CGG2013) height »,
   source `ESRI`). Retouche de six lignes.

## 5. Commandes et codes de sortie

Toutes les commandes sont lancées depuis la racine du worktree (`--manifest-path`).

- `cargo fmt --check` : 1 (4 écarts dans `tests/test_io_wkt2.rs`, avant application).
- `cargo fmt` puis `cargo fmt --check` : 0.
- `cargo clippy --all-targets --all-features -- -D warnings` : 101 (deux `bool` en paramètre, `check_epsg_4326` et `check_projected`, supprimés depuis) puis 0.
- `cargo test --all-features --test test_io_wkt2` : 101 avant le marquage des tests ouverts, puis 0 : 20 réussis, 21 ignorés.
- `UPDATE_DOCS=1 cargo test --test docs_in_sync --test progress_svg` : 0 (README.md, ROADMAP.md, docs/UNIT-TRIAGE.md et docs/progress.svg régénérés).
- `cargo test --all-features --no-fail-fast` : voir la section 6.

## 6. Suite complète

`cargo test --all-features --no-fail-fast` : code 0. Somme des binaires :
820 réussis, 0 échec, 32 ignorés. Aucun test qui passait avant ne régresse
(aucun échec ni panique dans la sortie). Les 21 ouverts de ce lot sont
comptés parmi les 32 ignorés.
