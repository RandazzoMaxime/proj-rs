# Fiche d'état — listes `projinfo --summary` du gros corpus (lot Q2b)

Lot : cas A (candidats « ballpark vertical », composé → géographique) et B (jambe
« axis order change (geographic3D horizontal) » absente dans les pivots) de
`tests/parity/wide/known.tsv`. Aucun correctif conservé : deux tentatives, les deux annulées.

## Mesure de base

- 38 listes lues ; 35 diffèrent de PROJ 9.9.0, 3 sont identiques (`0640`, `0750`, `0851`).
- Registre `tests/parity/wide/known.tsv` : 131 lignes, toutes de ces 35 listes.
- La consigne parlait de 32 listes ouvertes ; la mesure donne 35. La mesure fait foi.

## Tentative 1 — garde d'intermédiaire sur l'appel inverse (annulée)

Port : `src/iso19111/factory.rs`, bloc « geocentric CRS to compound CRS is the inverse »
(`create_candidates`, vers la ligne 6948). L'appel inverse `create_candidates(target, source)`
posait `GUARD_DATUM_INTERMEDIATE` sur tout son sous-arbre. Test : ne plus le poser.

Effet mesuré sur le gros corpus (`cargo test --test parity parity_wide -- --ignored`, 1656 s) :

- lignes fermées : `0107` (4/4), `0237` (2/2), `0057` (lignes 1, 2), `0142` (lignes 3, 4), `0257` (ligne 3) ;
- écarts nouveaux, donc échec du test : `0142` lignes 5, 6, 7 (PROJ 4 lignes, port 7) et
  `0257` lignes 1, 4, 5 (PROJ 2 candidats, port 4). Ces listes étaient identiques sur ces lignes.

Cause de la régression : le port bloque la recherche par CRS intermédiaires, puis PROJ
la fait dans le sous-appel. Les candidats ajoutés pour `0142` et `0257` passent par
`ETRF2014` / `ETRF96` → `ETRS89-NLD [AGRS2010]`, et PROJ ne les produit pas. Dans PROJ, la recherche
intermédiaire imbriquée est conditionnée par la présence de résultats directs non filtrés
(`coordinateoperationfactory.cpp:4449-4457`). `0107` et `0237` n'ont pas de résultat direct ;
`0142` et `0257` en ont. Le port ne reproduit pas cette condition dans l'appel imbriqué.
Piste pour la tentative suivante : reproduire la condition de 4449-4457 dans l'appel
imbriqué avant de retirer le garde.

## Tentative 2 — jambe de troisième rang par `geodetic_operations` (annulée)

Port : `pivot_leg` (`src/iso19111/factory.rs`) prenait la conversion EPSG:9602 nue pour une
jambe géographique ↔ géocentrique. Test : passer par `geodetic_operations` (port de
`createOperationsGeodToGeod`, `coordinateoperationfactory.cpp:5300-5390`), premier résultat.

Effet : aucun, sur les 38 listes (sortie identique à la base, 3 identiques).
Les listes B (`0205`, `0735`, `0785`) ne passent pas par cette jambe : le candidat de PROJ
vient de `createOperationsToGeod`. Annulée faute d'effet mesuré.

## Catégorie B — cause pour `0205`, `0735`, `0785`, `0128`

PROJ construit le candidat `… + axis order change (geographic3D horizontal) + Conversion from X
(geographic) to X` avec une jambe dont le CRS intermédiaire est nommé `X (geographic)`, en lat/lon.
Cette jambe sort de `createOperationsGeodToGeod`, branche des datums différents
(`coordinateoperationfactory.cpp:5354-5370`) : `interm_crs` lon/lat puis `createOperations(interm,
cible)`. Le port ne construit pas ce candidat : le chemin pivot n'atteint pas cette jambe
(le candidat `ETRS89-FRO [2008] … WGS 84 (Transit)` manque entièrement). Non résolu.

## Reste ouvert

- A : `0142`, `0257`, `0567` (et `0434`, `0679`, `0829`, non analysées) ; voir la tentative 1.
- B : `0205`, `0735`, `0785`, `0128` ; voir ci-dessus.
- E (emprise EPSG:9602) : non abordée dans ce lot.
- C et D : hors lot.
