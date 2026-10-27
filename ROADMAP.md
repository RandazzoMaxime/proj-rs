# Feuille de route

Chaque ligne est un lot : une pull request, un sujet, une fiche de port si
le lot traduit du C ou du C++. L'ordre de fusion est l'ordre du tableau.
Personne d'autre que le mainteneur ne fusionne (voir
[CONTRIBUTING.md](CONTRIBUTING.md)).

## Pile en cours de relecture

| Lot | Sujet | Source PROJ 9.4 |
|---|---|---|
| 0 | Gouvernance, CI, lints | — |
| 1 | Ellipsoïde, erreurs, réduction de longitude | paramètres `a`, `rf` de PROJ |
| 2 | Transverse Mercator exact et UTM | `src/projections/tmerc.cpp` (`PODER_ENGSAGER`) |
| 3 | Lambert conforme conique et Lambert-93 | `src/projections/lcc.cpp`, `tsfn.cpp`, `phi2.cpp` |
| 4 | Lot AVX2 du Transverse Mercator et chrono | `forward_batch` de `tmerc.cpp` |
| 5 | Lot AVX2 de Lambert-93 | `lcc_e_forward` par paquets de 4 |
| 6 | Mercator et Web Mercator | `src/projections/merc.cpp` |
| 7 | Helmert à 7 paramètres | `+proj=helmert` |

## Lots ouverts à la communauté

Réclamer le lot dans une issue avant d'ouvrir la PR. Rester sur le
comportement de PROJ 9.4, avec une fiche `docs/PORTING-*.md`.

| Lot | Sujet | Source PROJ |
|---|---|---|
| 8 | Transverse Mercator approché (Evenden / Snyder) | branche `approx` de `tmerc.cpp` |
| 9 | Moteur de pipeline (`+proj=pipeline`, un step à la fois) | `src/pipeline.cpp` |
| 10 | Conversion géodésique ↔ cartésien géocentrique | `src/conversions/cart.cpp` |
| 11 | Lambert azimutal égal-aire | `src/projections/laea.cpp` |

Hors périmètre tant qu'un lot ne le réclame pas explicitement : base EPSG,
grilles NTv2, analyse WKT / PROJJSON, les projections restantes. Un lot ne
les ajoute pas « pendant qu'on y est ».
