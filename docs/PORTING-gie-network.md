# Fiche de portage : gie, gie self-tests, projsync, réseau

Audit détaillé : `docs/AUDIT-gie-network.md`.

## Carte

| Source PROJ 9.9.0 | Rust |
|---|---|
| `src/apps/gie.cpp` | `src/gie.rs` (runner, bibliothèque `proj_rs::gie`), `src/bin/gie.rs` (options, usage, `-l`, résumé) |
| `test/unit/gie_self_tests.cpp` | `tests/gie_self_tests.rs` (15 fonctions, 14 tests C++) |
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
- `proj_trans` sur un objet CRS : une erreur `ProjError`, pas le code
  `PROJ_ERR_INVALID_OP_ILLEGAL_ARG_VALUE` exact.
- `proj_trans_generic` (tableaux strides) : porté comme `transform_array`
  sur des points, qui est la forme Rust équivalente.
- Changement de contexte (`proj_context_set`) : pas d'équivalent, un objet
  Rust ne porte pas de contexte.
- Porte de latitude : une correction du moteur (voir `latitude_gate`) est
  nécessaire, la crate donnait `OutsideDomain` pour une latitude de 95° en UTM.

## Commandes

```
cargo test --offline --test gie_cli --test gie_run --test gie_self_tests
```
