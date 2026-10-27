## Lot

Lot 5 — `forward_batch` de Lambert-93 en AVX2. Le scalaire du lot 3 ne change pas.

## Comportement PROJ conservé

Même cône que `lcc_e_forward`. Un point refusé par `forward` est écrit `NaN`, comme `HUGE_VAL`, et le lot continue. Les latitudes à moins de `1e-10` radian du pôle restent sur le chemin scalaire, celui qui teste `phi * n <= 0`.

`batch_matches_scalar_within_ten_nanometres` compare le lot au scalaire, lui-même calé sur `cs2cs`, sous 10 nm. `eiffel_matches_cs2cs` relance toujours `cs2cs`.

## Écarts volontaires

- `pow(ts, n)` devient `exp(n · ln(ts))` pour appeler libmvec.
- Quatre `sin`, `cos`, `atanh`, `exp` ou `log` par appel.
- Le `unsafe` reste dans `src/simd.rs` et dans `forward4`, appelé seulement après détection d'AVX2 et de FMA.

## Vérification

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Les trois passent.

## Fusion

Empilée sur le lot 4. @RandazzoMaxime fusionne dans l'ordre. Pas de merge automatique.
