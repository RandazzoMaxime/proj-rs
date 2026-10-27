## Lot

Lot 3 — port de `lcc.cpp`, `tsfn.cpp`, `msfn.cpp` et `phi2.cpp` (PROJ 9.4).

Fiche : `docs/PORTING-lcc.md`.

## Comportement PROJ conservé

Cône ellipsoïdal de PROJ. Lambert-93 : GRS80, parallèles 49° et 44°, origine 46,5°N 3°E, faux origine (700 000 m, 6 600 000 m).

`cs2cs` (GRS80), tolérance 1e-6 m : origine, tour Eiffel, trois points en France. `tests/agree_cs2cs.rs` relance `cs2cs` au moment du test. L'itération de Newton de `pj_sinhpsi2tanphi` garde les comparaisons inversées : un NaN traverse, comme dans PROJ.

## Écarts volontaires

- Les huit arguments positionnels deviennent le struct `Cone`.
- Les messages d'erreur C deviennent `ProjectionError::InvalidParallel` ou `DegenerateCone`.
- Pas de cône sphérique.
- `Lcc::lambert93` panique seulement si les constantes officielles devenaient illégales.

## Vérification

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Les deux passent, y compris `eiffel_matches_cs2cs`.

## Fusion

Empilée sur le lot 2. @RandazzoMaxime fusionne dans l'ordre. Pas de merge automatique.
