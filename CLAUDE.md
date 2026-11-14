# CLAUDE.md

Port Rust de PROJ 9.9.0 (projections, pipelines, proj.db embarquée). Lire
[CONTRIBUTING.md](CONTRIBUTING.md) pour le processus de lot et de PR, et
[ROADMAP.md](ROADMAP.md) pour les sujets ouverts.

## Commandes

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Les tests de référence comparent au `cs2cs` de PROJ 9.9.0. Pour l'utiliser en
local, pointer `PROJ_RS_LIBPROJ_DIR` vers la lib de PROJ 9.9.0 (`build.rs`).
La CI définit aussi `PROJ_CS2CS_REQUIRED=1` et
`PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES`.

Sans PROJ installé (session cloud), `tools/build-reference.sh` compile
`cs2cs` et `projinfo` 9.9.0 depuis les sources du tag (2 min environ) pour
comparer n'importe quelle paire au vrai PROJ, pas seulement les cas des yaml.

## Règles de travail

- Personne ne pousse sur `main`, sauf l'administrateur du dépôt (Maxime Randazzo, avec ses
  droits admin). Tout autre changement passe par une branche et une PR. Ne pas fusionner :
  le mainteneur est seul à fusionner. Exception : quand l'administrateur le demande
  explicitement, Claude fusionne la PR dont la CI est verte (squash). Claude ne pousse
  jamais sur `main` lui-même et ne contourne aucun ruleset : un push direct refusé se
  règle par une PR.
- Claude travaille sur des branches `claude/<sujet>` (une par lot ou par test) et peut les
  créer sans demander.
- Un lot = une PR, un seul sujet. Ne pas mélanger deux lots dans une branche.
- Un port commence par les tests qui caractérisent le comportement PROJ, puis
  l'implémentation. Les valeurs attendues viennent de PROJ 9.9.0, jamais du
  code testé. Pas de mock de l'unité testée.
- Un écart volontaire vers PROJ se justifie dans `docs/PORTING-<module>.md`.
- Ne jamais renommer un cas de benchmark existant : `bench/history.jsonl` est
  indexé par nom.

## Fichiers générés

Ne pas éditer à la main les chiffres de `README.md` et `ROADMAP.md`, ni
`docs/UNIT-TRIAGE.md`. Après avoir retiré des lignes de
`docs/UNPORTED-UNIT.md` ou `docs/UNPORTED-CLI.md` :

```bash
UPDATE_DOCS=1 cargo test --test docs_in_sync
cargo test --test docs_in_sync
```

## Conventions de code

- Pas de `unwrap()` hors tests. `expect()` est autorisé.
- Pas de paramètre `bool` dans une signature publique : utiliser une enum.
- Pas de `unsafe` sans commentaire `// SAFETY:`, sauf besoin mesuré ou FFI.
- Les lints sont dans `Cargo.toml` (`[lints]`) et `clippy.toml`. Ne pas les
  désactiver avec `#[allow]` sans raison écrite.
- Pas de `print_stdout` ni `print_stderr` hors binaires CLI.
