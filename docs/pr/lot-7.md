## Lot

Lot 7 — Helmert à 7 paramètres, forme linéarisée de `+proj=helmert` (PROJ 9.4).

Fiche : `docs/PORTING-helmert.md`.

## Comportement PROJ conservé

Translations en mètres, rotations en secondes d'arc, échelle en ppm. `position_vector` et `coordinate_frame` (signe des rotations opposé). Les cinq points `cct` du test collent à 1e-6 m.

La rotation reste linéarisée : 3600 secondes d'arc ne deviennent pas un sinus, mais l'angle en radians. C'est le comportement de PROJ, pas une approximation ajoutée ici.

## Écarts volontaires

- Pas de pas de temps, pas de grille.
- Un paramètre non fini est `Err(ProjectionError::NonFinite)`.

## Vérification

```bash
cargo test --all-features helmert
cargo clippy --all-targets --all-features -- -D warnings
```

## Fusion

Empilée sur le lot 6. @RandazzoMaxime fusionne dans l'ordre. Pas de merge automatique.
