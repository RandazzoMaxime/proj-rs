# Port — rejet des WKT invalides (`wkt_parse.invalid*`)

Source : PROJ 9.9.0, `test/unit/test_io.cpp` (groupe `wkt_parse`, tests
`invalid*`) et `src/iso19111/io.cpp`, `buildCoordinateOperation`.

Ce lot porte 21 `TEST()` de `test_io.cpp`. Il ne touche pas aux CRS
dérivés, temporels, paramétriques ou aux `CoordinateMetadata`, qui restent
dans `docs/UNPORTED-UNIT.md`.

## Correspondance

| PROJ 9.9.0 `wkt_parse.*` | PROJ-RS `tests/test_io.rs` |
|---|---|
| `invalid` | `wkt_parse_invalid` |
| `invalid_SPHEROID` | `wkt_parse_invalid_spheroid` |
| `invalid_DATUM` | `wkt_parse_invalid_datum` |
| `invalid_ENSEMBLE` | `wkt_parse_invalid_ensemble` |
| `invalid_BBOX` | `wkt_parse_invalid_bbox` |
| `invalid_GEOGCS` | `wkt_parse_invalid_geogcs` |
| `invalid_UNIT` | `wkt_parse_invalid_unit` |
| `invalid_GEOCCS` | `wkt_parse_invalid_geoccs` |
| `invalid_CS_of_GEODCRS` | `wkt_parse_invalid_cs_of_geodcrs` |
| `invalid_CS_of_GEOGRAPHICCRS` | `wkt_parse_invalid_cs_of_geographiccrs` |
| `invalid_DYNAMIC` | `wkt_parse_invalid_dynamic` |
| `invalid_PROJCRS` | `wkt_parse_invalid_projcrs` |
| `invalid_PROJCS` | `wkt_parse_invalid_projcs` |
| `invalid_VERTCRS` | `wkt_parse_invalid_vertcrs` |
| `invalid_esri_VERTCS` | `wkt_parse_invalid_esri_vertcs` |
| `invalid_VERT_CS` | `wkt_parse_invalid_vert_cs` |
| `invalid_COORDINATEOPERATION` | `wkt_parse_invalid_coordinateoperation` |
| `invalid_CONCATENATEDOPERATION` | `wkt_parse_invalid_concatenatedoperation` |
| `invalid_BOUNDCRS` | `wkt_parse_invalid_boundcrs` |
| `invalid_TOWGS84` | `wkt_parse_invalid_towgs84` |
| `invalid_LOCAL_CS` | `wkt_parse_invalid_local_cs` |

Les tests C++ qui créent un CRS puis l'exportent en WKT2 (`invalid_BOUNDCRS`,
`invalid_COORDINATEOPERATION`, `invalid_CONCATENATEDOPERATION`) utilisent
l'export WKT2 2015 de Rust à la place. Pour `invalid_CONCATENATEDOPERATION`, le
`transf_1` du C++ est écrit en texte WKT, car `Transformation` n'a pas de
constructeur public ici.

## Correction de la bibliothèque

`COORDINATEOPERATION` en tête de WKT exige `SOURCECRS` et `TARGETCRS`, comme
`buildCoordinateOperation` en C++. Avant ce lot, l'absence de l'un des deux
était acceptée. Le contrôle est dans `src/iso19111/wkt_import.rs`, branche
`COORDINATEOPERATION` du dispatch de premier niveau.

Les `ABRIDGEDTRANSFORMATION` de `BOUNDCRS` gardent `SOURCECRS` facultatif,
comme dans le C++ (ils ne passent pas par `buildCoordinateOperation`). Les
`STEP` de `CONCATENATEDOPERATION` ne sont pas touchés.

## Écarts volontaires

Aucun sur le comportement testé. Le message d'erreur n'est pas comparé : les
tests C++ vérifient seulement le type d'exception (`ParsingException`), et Rust
renvoie une `FactoryError`.

## Commandes

```
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
PATH=<proj-ref>/bin:$PATH PROJ_CS2CS_REQUIRED=1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES cargo test --all-features --no-fail-fast
```

Résultat : clippy propre, suite complète verte. `wkt_parse_invalid*` : 22
tests passants (21 du lot et `wkt_parse_invalid_geogcrs_with_ensemble`, déjà
porté).
