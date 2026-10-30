# Port — WKT1 `Polar_Stereographic`, `Mercator_1SP` et unités PROJ (`wkt_parse.wkt1_*`)

Source : PROJ 9.9.0, `test/unit/test_io.cpp` (groupes `wkt_parse.wkt1_polar_stereographic_*`,
`wkt_parse.wkt1_Mercator_1SP_*`, `wkt_parse.wkt1_Spherical_Cross_Track_Height`) et
`src/iso19111/io.cpp` :

- `WKTParser::Private::buildProjection`, branche `Polar_Stereographic` : variante B
  si `latitude_of_origin` est présent et `scale_factor` vaut 1, variante A si la
  latitude vaut ±90, sinon chemin générique ;
- `WKTParser::Private::buildProjection`, paramètre `scale_factor` absent : avertissement
  `emitRecoverableWarning` et valeur 1 ;
- `WKTParser::Private::buildProjectedCRS`, CS par défaut quand il n'y a ni `CS` ni
  `AXIS` ;
- `CartesianCS::createNorthPoleEastingSouthNorthingSouth` et
  `createSouthPoleEastingNorthNorthingNorth` (`coordinatesystem.cpp`) ;
- `UnitOfMeasure::exportToPROJString` (`common.cpp`), qui choisit l'identifiant
  `+units=` par facteur de conversion.

Ce lot porte 8 `TEST()`. Il ne touche ni aux CRS WKT1 `Krovak` ou
`Transverse_Mercator_South_Orientated`, ni aux `PROJCS` sans projection
(`wkt1_projected*`), ni à l'import `EXTENSION["PROJ4", ...]`.

## Correspondance

| PROJ 9.9.0 `wkt_parse.*` | PROJ-RS `tests/test_io.rs` |
|---|---|
| `wkt1_Mercator_1SP_with_latitude_origin_0` | `wkt_parse_wkt1_mercator_1sp_with_latitude_origin_0` |
| `wkt1_Mercator_1SP_without_scale_factor` | `wkt_parse_wkt1_mercator_1sp_without_scale_factor` |
| `wkt1_polar_stereographic_latitude_of_origin_70` | `wkt_parse_wkt1_polar_stereographic_latitude_of_origin_70` |
| `wkt1_polar_stereographic_latitude_of_origin_minus_70` | `wkt_parse_wkt1_polar_stereographic_latitude_of_origin_minus_70` |
| `wkt1_polar_stereographic_latitude_of_origin_90` | `wkt_parse_wkt1_polar_stereographic_latitude_of_origin_90` |
| `wkt1_polar_stereographic_latitude_of_origin_90_scale_factor_1` | `wkt_parse_wkt1_polar_stereographic_latitude_of_origin_90_scale_factor_1` |
| `wkt1_polar_stereographic_scale_factor` | `wkt_parse_wkt1_polar_stereographic_scale_factor` |
| `wkt1_Spherical_Cross_Track_Height` | `wkt_parse_wkt1_spherical_cross_track_height` |

Les tests C++ comparent `exportToPROJString(PROJ_4)` avec `+type=crs`. Le Rust
appelle `ProjectedCRS::export_to_proj_string`, qui écrit la même suite. Les
axes sont lus sur `coordinate_system().axes()`, la direction en anglais minuscule
(`"north"`, `"south"`).

## API

Aucune API publique ajoutée. Les changements sont internes à
`src/iso19111/wkt_import.rs` et `src/iso19111/projstring.rs` :

| PROJ 9.9.0 | Rust |
|---|---|
| variante B de `buildProjection` (`Polar_Stereographic`) | `build_wkt1_projection`, drapeau `polar_variant_b` |
| avertissement `scale_factor` absent | `build_wkt1_projection`, appel à `recover` |
| CS par défaut des pôles (`buildProjectedCRS`) | `polar_pole_cs`, appelée depuis `build_projected` |
| `UnitOfMeasure::exportToPROJString` | `linear_unit_clause` appelle `linear_proj_unit` |

## Écarts volontaires

- **Axes polaires : deux méthodes seulement.** PROJ applique la même règle à
  `Lambert_Azimuthal_Equal_Area` (`EPSG_CODE_METHOD_LAMBERT_AZIMUTHAL_EQUAL_AREA`).
  Le crate ne fait pas correspondre ce nom WKT1 à une méthode EPSG, donc la règle
  n'a pas de cas d'entrée ; elle reste à ajouter si ce nom est porté.
- **Axes de Krovak et de Transverse Mercator South Orientated** : PROJ les pose
  dans la même branche `buildProjectedCRS`. Non portés dans ce lot ; les tests
  `wkt1_krovak_*` restent ouverts.
- **`setStrict(false)`** de PROJ est porté par `WktStrictness::Lenient`. Le
  cas `Mercator_1SP` ne dépend d'aucune tolérance supplémentaire de PROJ : `UNIT` et
  `AXIS` sont présents dans le WKT.
- **Avertissement `scale_factor`** : en strict, `recover` renvoie l'erreur comme
  `emitRecoverableWarning` lève `ParsingException`. Le message est celui de PROJ.

## Comportements conservés

- `Polar_Stereographic` avec `latitude_of_origin` et sans `scale_factor` (ou avec
  `scale_factor` égal à 1) devient la variante B, même à ±90. Le `scale_factor` ne
  figure pas dans la conversion et n'apparaît pas dans `+k=`.
- `Polar_Stereographic` avec un `scale_factor` différent de 1 et une latitude de
  ±90 reste la variante A, qui écrit `+k=`.
- Les axes par défaut d'un `PROJCS` sans `AXIS` : pôle nord (variante A à +90, ou
  variante B avec `latitude_of_origin` positive) donne Est et Nord en `south` ; pôle
  sud donne `north` pour les deux. Sans pôle, les axes restent est/nord.
- `+units=` vient du facteur de conversion (`pj_list_linear_units`). Une unité
  nommée `Meter` avec facteur 1 écrit `+units=m`, comme PROJ. Une unité sans
  identifiant PROJ n'écrit rien.

## Non portés dans ce lot

Restent dans `docs/UNPORTED-UNIT.md` (section `test_io.cpp`, 370 entrées), entre autres :

- `wkt_parse.wkt1_krovak_*`, `wkt_parse.wkt1_hotine_*`, `wkt_parse.wkt1_projected*`
  (CRS projetés avec `AXIS` ou `PROJ4` en extension) ;
- `wkt_parse.wkt1_esri_ups_*`, `wkt1_esri_krovak_*` et autres correspondances ESRI ;
- les groupes `io.projstringformatter*` et `json_import.*` ;
- `io.projparse_longlat_*` au-delà du lot précédent (PR #31).

## Vérification

Les commandes sont lancées dans le worktree `lot-unit-io-2`.

```
cargo fmt --check                                                  # ok
cargo clippy --all-targets --all-features -- -D warnings           # ok, aucune sortie
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 \
  PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
  cargo test --all-features --test test_io
# test result: ok. 84 passed; 0 failed; 0 ignored
cargo test --all-features --no-fail-fast                           # EXIT 0, aucun échec
#   un seul test ignoré, préexistant : src/projections/nkg.rs (réseau, PROJ_NETWORK=ON)
```

Le harnais `cli_yaml` réécrit `docs/UNPORTED-CLI.md` à chaque suite complète. Ce fichier
est restauré avant commit et n'est pas inclus dans le lot.

Recompte du ledger `test_io.cpp` : 378 → 370. Total ouvert : 998 → 990. Portés
`test_io` : 76 → 84 (README). Total « ported » : 354 → 362.
