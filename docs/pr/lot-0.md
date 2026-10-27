## Lot

Lot 0 — ouvrir PROJ-RS : gouvernance, licence MIT, CI, lints BeautifulRust.

## Comportement PROJ conservé

Aucun algorithme dans ce lot. Le crate expose seulement `VERSION`.

## Écarts volontaires

Aucun.

## Vérification

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Les trois commandes passent sur Rust 1.83.

## Fusion

Cette PR ne se fusionne pas toute seule. @RandazzoMaxime fusionne, dans
l'ordre des lots : 0, puis 1, puis 2, puis 3, puis 4.

La base est le commit initial vide. `main` de la forge temporaire contient
encore le premier jet, hors de cette pile. Après relecture, la branche
`cursor/lot-4-simd-bench-a775` est la ligne à publier.
