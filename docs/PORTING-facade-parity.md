# Port — façade : trous restants de la parité georust

Sources, PROJ 9.9.0, et mesures sur PROJ 9.8.1 (pyproj 3.8.0, `libproj.25.9.8.1`,
`proj_context_set_database_path` sur le `proj.db` embarqué). Pas de binaire 9.9.0 ici.
georust `proj` 0.31.0 : `new_known_crs` appelle `proj_create_crs_to_crs` puis
`proj_normalize_for_visualization` ; `def()` lit `proj_pj_info().definition`
(null → erreur).

- `src/iso19111/c_api.cpp` : `proj_as_wkt`, `proj_as_projjson`,
  `proj_normalize_for_visualization`, `proj_coordinate_metadata_create`.
- `src/iso19111/io.cpp` : import PROJJSON (`ProjectedCRS`, `GeographicCRS`).
- `src/4D_api.cpp` / `src/init.cpp` : `proj_pj_info`, `pj_datum_set`,
  `append_default_ellipsoid_to_paralist` (ellipsoïde `GRS80` hors pipeline, sans
  datum, ellipsoïde ni `no_defs`). Un pipeline d'une seule étape est aplati avant.
- `src/iso19111/operation/coordinateoperationfactory.cpp` :
  `createOperationsGeogToGeog` écrit un `Null geographic offset` pour deux CRS
  géographiques du même datum. `proj_create_crs_to_crs` avec une époque insère
  `+proj=set +v_4=<année>` aux deux bouts du pipeline.
- georust `src/proj.rs` : `test_projjson` (`EPSG:2230` → `EPSG:26946`),
  `test_metadata_creation` (époque sur `EPSG:4326`).

## Correspondance

| PROJ / georust | Rust |
|---|---|
| `proj_as_projjson` / `proj_as_wkt` d'une paire | `Proj::to_projjson`, `Proj::as_wkt` sur l'opération normalisée |
| import `ProjectedCRS` | `iso19111::json::read_projected` |
| `proj_pj_info().definition` | `Proj::def` |
| `proj_pj_info().id` / `.description` | `Proj::proj_info` |
| époque d'un seul côté | `proj=set v_4=<année>` dans le pipeline ISO |
| deux époques | `ProjCreateError` |

## Tests

Appels à l'API publique. Les nombres viennent de `proj_trans` / `proj_pj_info` /
`proj_as_projjson` / `proj_as_wkt` (WKT2:2019) mesurés sur PROJ 9.8.1, ou des
textes de `test/unit/test_io.cpp` déjà dans `tests/proj_export.rs`.

| Test | Ce qu'il fixe |
|---|---|
| `tests/georust_parity.rs` `test_projjson` | plus de `#[ignore]` : 25 tests, 0 ignoré (`--all-features`) |
| `tests/proj_export.rs` `transform_between_two_crs_exports_the_operation` | définition, nom, id, longueurs JSON 10958 et WKT 6238 de `EPSG:2230` → `EPSG:26946` |
| `tests/proj_export.rs` `projjson_of_a_projected_crs_roundtrips_through_the_facade` | import puis export du CRS projeté du test unitaire PROJ |
| `tests/proj_facade.rs` | `EPSG:4326` → `EPSG:2154` écrit un `ConcatenatedOperation` ; un PROJJSON projeté s'exporte |
| `tests/facade_info.rs` `pipeline_cart_and_init_definitions_follow_proj_pj_info` | pipeline d'une étape, `+proj=cart`, `+init=epsg:4326` |
| `tests/facade_info.rs` `a_single_crs_has_no_operation_definition` | `EPSG:4326` : définition nulle |
| `tests/facade_info.rs` `pair_definitions_follow_proj_pj_info` | `4326→4326`, `4326→3857`, `4269→4326` |
| `tests/proj_meta_facade.rs` `an_epoch_moves_itrf2014_to_itrf2008` | `EPSG:7789` → `EPSG:5332`, temps 0 / 2010 / 2020 |
| `tests/proj_meta_facade.rs` `two_epochs_are_refused` | deux époques |
| `tests/proj_meta_facade.rs` `an_epoch_on_a_static_projected_pair_keeps_the_undated_point` | `4326` daté → `3857`, point inchangé |
| `tests/proj_unit.rs` | `def()` d'un CRS n'est plus le code EPSG |

Point `7789` → `5332`, entrée `(4027893, 307045, 4919474)`, `proj_trans` sans
métadonnée (temps 0) : `4027892.7586375084 307044.98337904655 4919473.9066573335`.
Époque 2010 : `4027893.001519442 307045.00189385907 4919474.00230161`.
Époque 2020 : `4027893.0027278103 307045.0019859726 4919474.0027774535`.
`Transformer` de pyproj sur la même paire rend le point de 2010 : ce n'est pas
l'oracle sans époque.

Web Mercator `(2°, 49°)` : `222638.98158654713 6274861.394006575`.

## Écarts volontaires

- **`+init=`.** `proj_create("+init=epsg:4326")` est l'erreur 1025, même avec
  `proj.db`. pyproj en mode CRS répond
  `proj=longlat datum=WGS84 no_defs ellps=WGS84 towgs84=0,0,0`. Ici `+init=` est
  développé (`+proj=longlat +a=6378137.0 +rf=298.257223563000025 +no_defs`) et
  `def()` est le `proj_pj_info` de cette chaîne, qui conserve les 15 chiffres de
  `format_f64` (`rf=298.257223563000025`).
- **`+proj=cart`.** `proj_create` laisse `proj=cart ellps=WGS84`. Le mode CRS de
  pyproj le réécrit en `geocent`. On suit `proj_create`.
- **Pipeline.** Une étape est aplatie, puis `pj_datum_set` et l'ellipsoïde par
  défaut. Plusieurs étapes : les `+` tombent, pas de `pj_datum_set`, pas
  d'ellipsoïde ajouté.
- **CRS seul.** `proj_pj_info` a une définition nulle. `def()` est
  `ProjError::Definition`, pas le code.
- **Plusieurs opérations instanciables.** `EPSG:4269` → `EPSG:4326` (4 chez nous,
  comme le `id=unknown` de PROJ) : le texte exact
  `unavailable until proj_trans is called`, id `unknown`. Pas une opération choisie
  au hasard.
- **Même datum.** L'usine n'émet toujours pas le décalage nul (écart déjà décrit
  pour `projinfo` et `cs2cs`). La façade seule le synthétise quand la liste de
  candidats est vide, pour `EPSG:4326` → `EPSG:4326` :
  `proj=noop ellps=GRS80`. Un choix approximatif ou sans grille ne devient pas
  ce décalage.
- **Définition d'une paire.** C'est la chaîne après
  `proj_normalize_for_visualization`, comme georust. `EPSG:4326` → `EPSG:3857`
  n'a pas le `axisswap` d'entrée de l'ordre d'autorité. Le pipeline exécuté le
  garde : la façade échange les axes avant de l'appeler. Les deux chemins
  prennent la longitude d'abord.
- **Époque.** Une époque est appliquée, y compris sur un CRS statique
  (`EPSG:4326`), parce que `test_metadata_creation` de georust le fait. PROJ 9
  refuse une époque sur un datum statique. Deux époques : erreur de création.
  Si le pipeline ISO manque, une époque est une erreur de création, pas un point
  non daté. Sans époque, le temps reste 0 (pas NaN, pas `t_epoch`).
- **`anchor_epoch`.** Écrit dans le PROJJSON du datum. `deformation_model` est
  omis (le WKT `MODEL[]` le porte). L'oubli évitait l'erreur qui bloquait
  `EPSG:4326` → `EPSG:2154`.
- **JSON du CRS projeté.** L'écrivain met toujours `"type": "GeographicCRS"` sur
  la base. Le test unitaire PROJ retire cette ligne avant de comparer ; le test
  de façade fait de même. Un nom vide devient `unnamed`.
- **Précision de `proj_info`.** Reste `-1` pour une paire. L'exactitude PROJ
  (0 pour `2230`, 0,01 pour `7789`) n'est pas recopiée.
- **Longueurs `2230`.** 10 958 octets de PROJJSON et 6 238 de WKT2:2019, mesurés
  sur PROJ 9.8.1, obtenus aussi avec le `proj.db` 9.9 de la crate. Pas une
  égalité octet à octet exigée au-delà de la longueur, du type et du nom.

## Commandes

```text
cargo test --all-features --no-fail-fast --test facade_info --test proj_meta_facade \
  --test proj_export --test proj_facade --test georust_parity --test proj_unit
```

Sortie : 8 + 25 + 13 + 9 + 10 + 36 tests, 0 échec, 0 ignoré. Code de sortie 0.
georust : 25 passés, 0 `#[ignore]`.
