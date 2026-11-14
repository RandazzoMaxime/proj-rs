# Fiche de portage : gie, gie self-tests, projsync, réseau

Audit détaillé : `docs/AUDIT-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/apps/gie.cpp` | `src/gie.rs` (runner, bibliothèque `proj_rs::gie`), `src/bin/gie.rs` (options, usage, `-l`, résumé) |
| `test/unit/gie_self_tests.cpp` | `tests/gie_self_tests.rs` (15 fonctions, 14 tests C++) |
| `gie.info_functions` (`gie_self_tests.cpp`, l. 351-482) | `tests/gie_info_functions.rs` |
| `proj_dmstor`, `proj_rtodms2`, `proj_strtod` (`dmstor.cpp`, `rtodms.cpp`) | `src/dms.rs` (`proj_rs::dms`) ; `src/bin/support/dms.rs` garde les noms des commandes |
| `proj_info`, `proj_grid_info`, `proj_init_info` (`info.cpp`, `grids.cpp`) | `src/info.rs` (`proj_rs::info`) |
| `pj_get_def` pour une projection qui n'est pas une CRS (`proj_pj_info`) | `operation_definition`, branche de repli, `src/proj_info.rs` |
| `test/cli/test_gie.yaml` | `tests/gie_cli.rs` |
| `test/gie/*.gie` (ratchet) | `tests/gie_run.rs` |
| proj_roundtrip, proj_trans_array, proj_xy_dist, proj_list_* | `src/gie_api.rs` |
| fwd_prepare (latitude, `fwd.cpp`) | `latitude_gate` dans `src/engine.rs` |
| `src/apps/projsync.cpp`, `test/cli/test_projsync.sh` | non porté |
| `test/unit/test_network.cpp` | non porté |

## Écarts

- `gie -o` : `gie.cpp` ouvre le fichier de sortie en lecture (`"rt"`), ce qui
  échoue pour un nouveau fichier. Le port écrit le rapport dans le fichier.
- `gie -vv` : pas de compteurs roundtrips / échecs séparés (la bibliothèque
  renvoie un total par fichier).
- `gie` : les erreurs de structure `<gie>` ne sont pas signalées.
- `proj_trans_generic` (tableaux strides) : porté comme `transform_array`
  sur des points, qui est la forme Rust équivalente.
- Changement de contexte (`proj_context_set`) : pas d'équivalent, un objet
  Rust ne porte pas de contexte.
- Porte de latitude : une correction du moteur (voir `latitude_gate`) est
  nécessaire, la crate donnait `OutsideDomain` pour une latitude de 95° en UTM.
- `proj_info().release` : « 9.9.0 » ici, « Rel. 9.9.0 (hash), date » dans PROJ.
  Le test vérifie seulement qu'il est non vide, comme le C.
- `proj_info().searchpath` : les répertoires par défaut sont `PROJ_DATA` (ou
  `PROJ_LIB`, ou `data/grids`), sans le répertoire utilisateur inscriptible de
  PROJ. La fonction ne lit aucune variable de contrôle : le `putenv` du C n'a pas
  d'équivalent.
- `proj_rtodms2` : la taille du buffer C `sizeof_s` est passée comme `max_len =
  sizeof_s - 1` (caractères sans le NUL terminal). Le retrait des zéros de
  `rtodms.cpp` s'applique au texte tronqué, comme dans le C.
- `DmsSign` : le signe `pos != 0, neg == 0` du C (sans lettre pour les négatifs)
  n'a pas d'équivalent ; les commandes ne l'emploient pas. Une lettre NUL n'écrit
  rien, comme le zéro du C.
- `proj_dmstor` : prend `&[u8]`, pas `&str`, pour lire le degré ISO 8859-1 (`0xB0`).
- `proj_grid_info` : `None` au lieu de `format = "missing"` et `filename = ""`.
  `filename` vaut `None` si le nom ne se résout pas en fichier local (le C le
  laisse vide aussi).
- `proj_init_info` : `None` au lieu de chaînes vides. `epsg` et `IGNF` n'ont pas
  de fichier ici : `filename` vaut `None`, les métadonnées viennent de la base.
- Bloc TIFF de `proj_grid_info` : la référence de build (`target/proj-reference`)
  est compilée sans TIFF (`ENABLE_TIFF=OFF`), donc son test C n'exécute pas ce
  bloc. La crate lit les GeoTIFF : le bloc tourne ici, avec les littéraux du C
  comme seul oracle disponible (pas de libtiff pour recompiler la référence).
- `proj_pj_info().definition` d'une projection nue (`+proj=utm`, `+proj=august`) :
  `Proj::def` renvoie maintenant ses paramètres (avec `ellps=GRS80` par défaut,
  comme `pj_init`), au lieu d'une erreur. Les chaînes qui sont des CRS gardent
  leur reconstruction ; un datum inconnu reste une erreur.

## Commandes

```
cargo test --offline --test gie_cli --test gie_run --test gie_self_tests --test gie_info_functions
```
`gie.proj_trans_with_a_crs` : porté. `proj_trans` sur un CRS renvoie `Err(ProjError::NotATransform)` (C++ : `HUGE_VAL`, `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE`).
Le code vient de `ProjError::code()` (`src/errno.rs`) ; la variante `Conversion` donnait `PROJ_ERR_COORD_TRANSFM` (2048), écart corrigé.
Test : `tests/gie_self_tests.rs::trans_with_a_crs_is_an_error`, écrit avant le correctif (échec : 2048 au lieu de 1027).
