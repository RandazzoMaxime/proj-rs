## Lot

Lot 1 — types partagés, avant toute projection.

## Comportement PROJ conservé

- `+a` et `+rf` : WGS 84 (`298.257223563`) et GRS 80 (`298.257222101`), demi-grand axe 6 378 137 m.
- `e² = f (2 − f)`, aplatissement troisième `n = f / (2 − f)`.
- `adjlon` ramène la longitude dans ]−π, π], exposé en degrés par `Geographic::normalized`.

## Écarts volontaires

- Un demi-grand axe non positif ou un aplatissement inverse ≤ 1 est `Err(EllipsoidError)`, pas un ellipsoïde à moitié initialisé.
- Les champs dérivés (`es`, `e`, `n`, `ra`) sont privés.

## Vérification

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Sept tests. Clippy `-D warnings` est vert.

## Fusion

Empilée sur le lot 0. @RandazzoMaxime fusionne le lot 0 d'abord. Pas de merge automatique.
