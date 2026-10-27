# Port — Lambert conforme conique

Source : PROJ 9.4, `src/projections/lcc.cpp`, `src/tsfn.cpp`, `src/msfn.cpp`,
`src/phi2.cpp`. Lambert-93 (EPSG:2154) est ce cône sur GRS80 :
parallèles 49° et 44°, origine 46,5°N 3°E, faux origine (700 km, 6 600 km).

## Correspondance d'API

| PROJ 9.4 | PROJ-RS |
|---|---|
| `PJ_PROJECTION(lcc)` | `Lcc::try_new` |
| paramètres EPSG:2154 | `Lcc::lambert93` |
| `lcc_e_forward` + `fwd_finalize` | `Lcc::forward` / `forward_rad` |
| `lcc_e_inverse` | `Lcc::inverse` |
| `pj_tsfn`, `pj_msfn`, `pj_phi2`, `pj_sinhpsi2tanphi` | fonctions privées homonymes |
| erreur de parallèles (`|lat_1 + lat_2| = 0`, `|lat| = 90°`) | `ProjectionError::InvalidParallel` |
| `n = 0` ou dénominateur nul | `ProjectionError::DegenerateCone` |
| pôle du mauvais côté (`phi * n <= 0` à ±90°) | `ProjectionError::OutsideDomain` |
| chaîne `&'static str` qu'un port naïf renverrait | enum `ProjectionError` |

## Correspondance de tests

| Comportement PROJ (`cs2cs`, GRS80) | Test Rust |
|---|---|
| Origine (3°, 46,5°) → (700 000, 6 600 000) | `proj_origin_is_false_origin` |
| Tour Eiffel et trois points en France | `proj_eiffel_and_samples` |
| Aller-retour | `roundtrip_france` |
| `lat_1 = 45`, `lat_2 = -45` | `rejects_parallel_sum_zero` |

## Écarts volontaires

- Pas de cône sphérique (`es = 0`). `Ellipsoid` refuse un aplatissement nul.
  PROJ a une branche `tan` à la place de `pj_tsfn` dans ce cas.
- Les échecs sont des `Result`, pas des messages anglais posés via `proj_errno`.
- `Lcc::lambert93` panique si les constantes officielles devenaient illégales.
  Ce n'est pas un chemin d'entrée utilisateur.
- Le lot par paquets écrit `NaN` là où `forward` renvoie `Err`. `pow(ts, n)`
  y devient `exp(n · ln(ts))`. `batch_matches_scalar_within_ten_nanometres`
  garde l'écart sous 10 nm. Les points à moins de `1e-10` rad du pôle
  restent sur le chemin scalaire.

## Quirks conservés

- Seuil `1e-10` (`EPS10`) pour « parallèle à 90° » et « somme nulle ».
- Cône sécant dès que `|phi1 - phi2| >= EPS10`, sinon `n = sin(phi1)`.
- Itération de Newton de `pj_sinhpsi2tanphi` : 5 tours maximum, mêmes seuils
  (`sqrt(eps) / 10`, `2 / sqrt(eps)`, bascule à `|tau'| > 70`). Si elle
  n'a pas convergé, PROJ pose une errno et renvoie quand même `tau`. On
  garde ce `tau`. Le noter évite qu'une « correction » change les bords.
- `atan2(x, y)` (pas `atan2(y, x)`) dans l'inverse, comme le C.

## Vérification

```bash
cargo test --all-features lcc
cargo clippy --all-targets --all-features -- -D warnings
```

Références produites par `cs2cs` de PROJ 9.4.0, tolérance 1e-6 m.
