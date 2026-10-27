## Lot

Lot 2 — port de `src/projections/tmerc.cpp` (PROJ 9.4), algorithme `PODER_ENGSAGER`.

Fiche : `docs/PORTING-tmerc.md`.

## Comportement PROJ conservé

Même série d'ordre 6 que `setup_exact` / `exact_e_fwd` / `exact_e_inv`, y compris le seuil `|Ce| <= 2.623395162778` et l'ordre de sommation de Clenshaw.

Points confrontés à `cs2cs` (PROJ 9.4.0), tolérance 1e-6 m :

- méridien 3°, latitude 48° → est 500 000 m
- équateur sur le méridien central → nord 0
- tour Eiffel (2,2945° ; 48,858222°) → 448 251,818 m ; 5 411 935,123 m
- trois points dans le fuseau 31N
- hémisphère sud : 10 000 000 m à l'équateur, et le point (3°, −10°)

L'aller-retour dans le fuseau reste sous 1e-11 degré.

## Écarts volontaires

- `HUGE_VAL` + errno devient `Err(ProjectionError::OutsideDomain)` ou `InvalidLatitude`. Pas de sentinelle sur le chemin scalaire.
- `+south` est `Hemisphere`, pas un `bool`.
- La zone est `UtmZone` (1..=60).
- `+approx` / Evenden–Snyder et la sphère ne sont pas dans ce lot.

## Vérification

```bash
cargo test --all-features tmerc
cargo clippy --all-targets --all-features -- -D warnings
```

Les deux passent.

## Fusion

Empilée sur le lot 1. @RandazzoMaxime fusionne dans l'ordre. Pas de merge automatique.
