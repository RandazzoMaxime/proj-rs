# Fiche d'état — listes `projinfo --summary` du gros corpus (lots Q2b à Q2d)

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

## Tentative 3 — mesure fine du garde retiré (annulée, lot Q2c)

Relecture de PROJ : le chemin inverse `geodSrc && compoundDst` (`coordinateoperationfactory.cpp:3782`)
appelle `createOperations(compound, geocentrique)` avec le même contexte, sans drapeau
`inCreateOperationsWithIntermediate`. La recherche intermédiaire (4449) s'y applique donc.
Le port la conditionne déjà sur le résultat direct vide (`direct_unfiltered_empty`). La condition
manquante n'est pas identifiée.

Mesure, garde retiré seul (`projinfo --summary`, lignes comparées à la référence) :

| Liste | Base : port / PROJ (lignes différentes) | Garde retiré : port / PROJ (lignes différentes) |
|---|---|---|
| `0107` | 2 / 5 (1) | 5 / 5 (aucune) |
| `0237` | 2 / 3 (1) | 3 / 3 (aucune) |
| `0057` | 3 / 4 (1, 2, 3) | 4 / 4 (3, 4) |
| `0142` | 3 / 4 (1, 3) | 7 / 4 (1, plus trois candidats en trop) |
| `0257` | 3 / 3 (3) | 5 / 3 (1, deux candidats en trop) |
| `0567` | 3 / 8 (1, 3) | 7 / 8 (1, 3 à 7) |

Les trois candidats en trop de `0142` ont une aire `4859`, une précision de 0,34 m et un préfixe
« Conversion from ETRF2014 (geocentric) to ETRF2014 (geog3D) + Inverse of Conversion … » : aller
puis retour par la même conversion. PROJ ne les liste pas. Cause non établie (hypothèse : un pas
géographique de même datum passe dans la recherche imbriquée). Le garde reste en place.

Piste suivante : tracer, par instrumentation temporaire, quelle branche du port produit ces
candidats, avant toute nouvelle modification du garde.

## Catégorie B — cause pour `0205`, `0735`, `0785`, `0128`

PROJ construit le candidat `… + axis order change (geographic3D horizontal) + Conversion from X
(geographic) to X` avec une jambe dont le CRS intermédiaire est nommé `X (geographic)`, en lat/lon.
Cette jambe sort de `createOperationsGeodToGeod`, branche des datums différents
(`coordinateoperationfactory.cpp:5354-5370`) : `interm_crs` lon/lat puis `createOperations(interm,
cible)`. Le port ne construit pas ce candidat : le chemin pivot n'atteint pas cette jambe
(le candidat `ETRS89-FRO [2008] … WGS 84 (Transit)` manque entièrement). Non résolu.

## Lot Q2d — six causes trouvées par traçage, corrigées

Traçage : PROJ 9.9.0 recompilé avec `TRACE_CREATE_OPERATIONS` et `ENABLE_TRACING`, sondes
temporaires dans `filterAndSort`, `createComputeMetadata` et les `catch` des constructions
PROJ-based ; port instrumenté dans une copie jetable. Chaque cause a son test de
caractérisation (valeurs PROJ, rouge sur l'ancien code) dans `tests/pivot_candidate_set.rs` ou
`tests/compound_candidate_set.rs`.

1. **Chaîne PROJ écrite à la construction.** `PROJBasedOperation::create`
   (projbasedoperation.cpp:115-130) écrit la chaîne PROJ du candidat composé ; une méthode que
   PROJ n'écrit pas (`Geographic3D to Depth (gtx)` 1121, `Time-specific Position Vector` 1065…)
   lève une exception et le candidat est abandonné. `proj_based_operation` (compound.rs) rend
   désormais `None` dans ce cas.
2. **Supersession dans `createBetweenGeodeticCRSWithDatumBasedIntermediates`**
   (factory.cpp:8576-8658) : une jambe remplacée par une autre jambe du lot est retirée
   (`superseded_by_listed`, factory.rs).
3. **Emprise du pont géographique ⇄ géocentrique** (EPSG:9602) : une `Conversion` chez PROJ,
   donc d'emprise `World` (`getExtent`, oputils.cpp:464). `operation_extent` la traite ainsi.
4. **Géographique ⇄ géocentrique de datums différents après un résultat du registre** :
   `createOperationsGeodToGeod` passe par `X (geographic)` et le `createOperations` complet
   (coordinateoperationfactory.cpp:5314-5331), pas seulement par un décalage approximatif.
5. **Ensemble de datums** : `datum_identity` garde l'identifiant de l'ensemble
   (`DatumEnsemble::asDatum`, datum.cpp:1943-1963). GR96 est EPSG:1421, pas le datum déprécié
   EPSG:6747 que son nom trouvait.
6. **Géocentrique → composé** : l'inverse de `createOperations(composé, géocentrique)` avec le
   même contexte (coordinateoperationfactory.cpp:3782-3786), sans la garde
   `GUARD_DATUM_INTERMEDIATE`. Et `hasIdentifiers` lit les étapes d'une opération enchaînée
   (coordinateoperationfactory.cpp:3110-3122) dans le choix des opérations verticales.

Au passage : `Geographic3D offsets` refuse un CRS composé (singleoperation.cpp:3973-4002), avec
les messages exacts de PROJ ; une opération sans chaîne PROJ n'est donc plus fusionnée comme
doublon par `removeDuplicateOps` (`0934`).

## Reste ouvert

Grand corpus après le lot Q2d : 0 écart nouveau, 116 des 131 lignes du registre fermées. Restent
15 lignes de 6 listes :

- `0567` : composé → géocentrique, 4 candidats ETRF perdus dans `bound_to_geod` ; le même calcul
  avec l'intermédiaire lu en WKT les trouve (contexte ou CRS intermédiaire à comparer).
- `0737` : le pivot de datum ne parcourt pas les paires de CRS IGNF dans l'ordre de PROJ.
- `0057` : un candidat vertical par une autre variante (offset nul 2D/3D contre ballpark).
- `0128` : chemin ESRI (`GCS_MONREF_1997`) au lieu de `axis order change + X (geographic)`.
- `0275`, `0494` : « World. » au lieu de « World » sur une variante du chemin `X (geographic)`.
