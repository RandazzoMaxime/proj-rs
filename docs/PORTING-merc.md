# Port — Mercator

Source : PROJ 9.4, `src/projections/merc.cpp`.

Deux modèles, comme PROJ : la sphère (`a = b`) et l'ellipsoïde.

## Correspondance d'API

| PROJ 9.4 | PROJ-RS |
|---|---|
| `+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +k=1` | `Mercator::web` |
| `+proj=merc +ellps=…` | `Mercator::try_ellipsoid` |
| `merc_e_forward` / sphérique | `Mercator::forward` |
| inverse | `Mercator::inverse` |
| `\|phi\|` à `1e-10` d'un pôle | `ProjectionError::OutsideDomain` |

`lat_ts` autre que 0 n'est pas dans ce lot : Web Mercator et le Mercator
ellipsoïdal standard l'ont à 0. Le noter évite d'inventer un facteur
d'échelle au parallèle sécant sans test.

## Correspondance de tests

| `cs2cs` | Test |
|---|---|
| Web, origine → (0, 0) | `proj_web_origin_is_zero` |
| Web, tour Eiffel, (3°, 46,5°), (−4°, 48°) | `proj_web_samples` |
| Ellipsoïde WGS 84, mêmes abscisses, ordonnées différentes | `proj_ellipsoid_differs_in_northing` |
| Aller-retour | `roundtrip` |
| Pôle | `pole_is_outside_domain` |

## Écarts volontaires

- La sphère n'est pas un `Ellipsoid` : `try_new` refuse un aplatissement nul.
  `Mercator::web` porte le rayon tout seul.
- `HUGE_VAL` au pôle devient `Err(ProjectionError::OutsideDomain)`.

## Quirks conservés

- Abscisse `k0 · λ` dans les deux modèles. Seule l'ordonnée change.
- Ordonnée ellipsoïdale `−k0 · ln(tsfn)`, identique à PROJ.
- Seuil `1e-10` rad avant le pôle.

## Vérification

```bash
cargo test --all-features merc
cargo clippy --all-targets --all-features -- -D warnings
```

Références `cs2cs` de PROJ 9.4.0, tolérance 1e-4 m (les sorties `cs2cs -d 6`
sont au micromètre ; 1e-4 m laisse de la marge sans masquer une formule fausse).
