# Port — Transverse Mercator exact et UTM

Source : PROJ 9.4, `src/projections/tmerc.cpp`, algorithme `PODER_ENGSAGER`
(`setup_exact`, `exact_e_fwd`, `exact_e_inv`). C'est le défaut
`tmerc_default_algo` de `proj.ini`, donc le chemin de `+proj=utm` sans
`+approx`.

Le lot 2 porte le chemin scalaire. Le lot 4 ajoute le traitement par paquets.

## Correspondance d'API

| PROJ 9.4 | PROJ-RS |
|---|---|
| `PJ_PROJECTION(utm)` + `+zone` + `+south` | `Tmerc::utm(UtmZone, Hemisphere, Ellipsoid)` |
| `PJ_PROJECTION(tmerc)` / `etmerc` | `Tmerc::try_new` |
| `pj_fwd` (radians, faux origine appliqué par `fwd_finalize`) | `Tmerc::forward` (degrés) et `Tmerc::forward_rad` |
| `pj_inv` | `Tmerc::inverse` |
| `HUGE_VAL` + `PROJ_ERR_COORD_TRANSFM_OUTSIDE_PROJECTION_DOMAIN` | `Err(ProjectionError::OutsideDomain)` |
| latitude hors [−π/2, π/2] | `Err(ProjectionError::InvalidLatitude)` |
| `+zone` invalide | `UtmZone::try_new` → `Err(ProjectionError::InvalidZone)` |
| `+south` | `Hemisphere::South` (pas un `bool`) |
| `gatg`, `clens`, `clenS`, `setup_exact` | fonctions privées du même nom de rôle |

`fwd_finalize` multiplie par `a` et ajoute `x0` / `y0`. C'est dans `forward_rad`,
pas dans la série.

## Correspondance de tests

| Comportement PROJ | Test Rust |
|---|---|
| Faux est 500 000 m sur le méridien 3°, latitude 48° (`cs2cs`) | `proj_false_easting_on_central_meridian` |
| Nord nul à l'équateur sur le méridien central | `proj_equator_is_origin_of_northing` |
| Tour Eiffel, valeurs `cs2cs` à 1e-8 m près dans la référence | `proj_eiffel_tower` |
| Trois points intérieurs de zone, mêmes valeurs `cs2cs` | `proj_sample_points` |
| `+south` : 10 000 000 m à l'équateur, et un point à −10° | `proj_south_false_northing` |
| Aller-retour (PROJ le garantit au nanomètre près dans le fuseau) | `roundtrip_matches_forward` |
| Zone 0 / 61, latitude 91° | `rejects_bad_zone_and_latitude` |

`tests/agree_cs2cs.rs` relance `cs2cs` sur la tour Eiffel. Le lot par paquets
écrit `NaN` là où le scalaire renvoie `Err`, pour coller à `HUGE_VAL` sans
arrêter le lot. Le chemin AVX2 remplace `hypot` par `sqrt` sur des arguments
bornés par 1 ; `batch_matches_scalar_within_ten_nanometres` garde l'écart
sous 10 nm.

## Écarts volontaires

- Le chemin scalaire renvoie `Result`. PROJ renvoie `HUGE_VAL` et pose `errno`.
  L'information est la même : le point est hors domaine, ou la latitude est
  illégale. Il n'y a pas de valeur sentinelle dans l'API Rust.
- `+approx` / `+algo=auto` / Evenden–Snyder ne sont pas portés. Ce n'est pas
  le défaut de PROJ 9.4 sur cette installation.
- La sphère (`es = 0`) n'existe pas : `Ellipsoid::try_new` refuse un
  aplatissement nul. PROJ bascule alors sur l'algorithme approché.
- `+over` (ne pas ramener la longitude) n'est pas exposé. `adjlon` est
  toujours appliqué, comme le défaut de PROJ.

## Quirks conservés

- Seuil `|Ce| <= 2.623395162778` recopié tel quel.
- Ordre 6 des séries et coefficients de `setup_exact`, y compris les
  corrections historiques (`n^5` de `cgb[2]` et `cgb[3]`).
- `clenS` accumule partie réelle et imaginaire dans le même ordre que le C
  (l'indice haut est consommé en premier).
- Les identités trigonométriques du direct (`sin(2 Cn)` sans appeler `sin`
  une seconde fois) sont celles du fichier C, pas la forme « non optimisée »
  commentée dans PROJ.

## Vérification

```bash
cargo test --all-features tmerc
cargo clippy --all-targets --all-features -- -D warnings
```

Les valeurs de référence viennent de `cs2cs` (PROJ 9.4.0) sur cette machine,
tolérance 1e-6 m. L'écart réel observé sur ces points est bien en dessous.
