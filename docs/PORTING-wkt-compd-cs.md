# Fiche : tests WKT `COMPD_CS` et ESRI implicite marqués `open` (lot wkt-compd-cs)

Source : PROJ 9.9.0, `test/unit/test_io.cpp` (`COMPD_CS_*`, `implicit_compound_CRS_*`). Les valeurs
attendues sont celles des tests C++ (assertions de nom, d'axes et d'export WKT1), jamais celles du
code testé. Fichier Rust : `tests/test_io_wkt2.rs`.

## Bilan

| Test Rust | Raison `open` d'origine | État |
|---|---|---|
| `wkt_parse_compd_cs_non_conformant_horizontal_plus_horizontal_as_in_las` | COMPD_CS donne un CompoundCRS, PROJ un ProjectedCRS 3D | passe, `#[ignore]` retiré |
| `wkt_parse_compd_cs_non_conformant_horizontal_towgs84_plus_horizontal_as_in_las` | COMPD_CS donne un CompoundCRS, PROJ un BoundCRS | passe, `#[ignore]` retiré |
| `wkt_parse_compd_cs_horizontal_bound_geog_plus_vertical_ellipsoidal_height` | COMPD_CS donne un CompoundCRS, PROJ un BoundCRS | passe, `#[ignore]` retiré |
| `wkt_parse_compd_cs_horizontal_projected_plus_vertical_ellipsoidal_height` | COMPD_CS, PROJ replie la hauteur en ProjectedCRS 3D | passe, `#[ignore]` retiré |
| `wkt_parse_compd_cs_horizontal_geog_plus_vertical_ellipsoidal_height_non_metre` | COMPD_CS, PROJ replie la hauteur en GeographicCRS 3D | passe, `#[ignore]` retiré |
| `wkt_parse_implicit_compound_crs_geographic_with_ellipsoidal_height_esri` | VERTCS ESRI nommé `D_WGS_1984` au lieu de `WGS_1984` | passe, `#[ignore]` retiré |
| `wkt_parse_implicit_compound_crs_projected_with_ellipsoidal_height_esri` | COMPD_CS ESRI implicite, PROJ donne un ProjectedCRS 3D | passe, `#[ignore]` retiré |

Sept `#[ignore]` retirés au total (six `COMPD_CS`, un ESRI). Aucune assertion modifiée, aucun
fichier `src/` modifié.

## Cause

Les raisons `open` ont été écrites avec les tests dans le commit `a8c325d`. Le code qui les
satisfait est déjà présent à l'arbre courant :

- `src/iso19111/wkt_import.rs::build_compound` appelle `lax_compd_cs` (le `CompoundCRS::createLax`
  de PROJ, crs.cpp:5396) pour un `COMPD_CS` : la branche géographique équivalente promeut la
  première composante en 3D (`set_nonconformant`), la branche `VERT_CS` datum 2002 ajoute l'axe
  ellipsoïdal et attache le compound d'origine (`attach_compound`).
- `promote_bound_hub_to_3d` (`src/iso19111/crs_ops.rs`) traite le hub des `BoundCRS`, comme
  `BoundCRS::promoteTo3D` de PROJ.

Les sept tests passent sans correctif : sous `--include-ignored`, les huit tests
`wkt_parse_compd_cs_*` et `wkt_parse_implicit_compound_crs_*` (les sept ignorés plus
`wkt_parse_implicit_compound_crs_esri`, déjà actif) réussissent.

## Écarts

Aucun écart nouveau. Les assertions sont celles du C++ 9.9.0 (ex. `test_io.cpp:3811`, `:3880`
pour les BoundCRS, `:3994` pour le pied US, `:4122` pour l'ESRI).

## Commandes

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
PATH=/home/user/proj-rs/target/proj-reference/build/bin:$PATH \
PROJ_DATA=/home/user/proj-rs/target/proj-reference/build/data \
  cargo test --test test_io_wkt2 -- --include-ignored wkt_parse_compd wkt_parse_implicit_compound
cargo test --test test_io_wkt2
```
