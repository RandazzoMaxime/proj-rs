# Fiche d'état — listes `projinfo --summary` du gros corpus (lots Q2b à Q2e)

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

## Lot Q2e — cinq causes trouvées par traçage, six listes fermées

Même méthode que le lot Q2d : PROJ 9.9.0 recompilé avec `TRACE_CREATE_OPERATIONS`, port
instrumenté dans une copie jetable. Chaque cause a son test de caractérisation (valeurs PROJ,
rouge sur l'ancien code) dans `tests/compound_candidate_set.rs` ou `tests/pivot_candidate_set.rs`.

1. **`0567` et `0057`, identifiants construits par nom.**
   `createBetweenGeodeticCRSWithDatumBasedIntermediates` lit les identifiants des deux CRS par
   `buildCRSIds` (coordinateoperationfactory.cpp:1775-1870), qui donne à un CRS sans identifiant
   celui de son nom. Le port lisait les seuls identifiants : l'intermédiaire géographique 3D de
   `createOperationsToGeod` (`ITRF89`, sans identifiant) ne trouvait rien et les quatre candidats
   ETRF disparaissaient. Ce n'était ni l'emprise `World` de l'intermédiaire ni le contexte : le
   CRS lu en WKT était identifié par `resolve_crs` au premier niveau, pas l'intermédiaire.
   `0057` (`EPSG:7815 → EPSG:10747`, géocentrique vers composé) passe par le même intermédiaire :
   sa liste, différente à `24a50b1`, devient identique avec cette seule correction.
2. **`0737`, le pivot de datum accumule.** `createOperationsWithDatumPivot` ajoute
   les résultats de chaque paire à une liste `res` et ne s'arrête que si un résultat n'est pas
   fait de seules étapes `PROJ` (`hasResultSetOnlyResultsWithPROJStep`,
   coordinateoperationfactory.cpp:3136-3163). Une jambe `WGS 84 to WGS 84 (G873)` est de
   l'autorité `PROJ` : les paires IGNF suivantes ajoutent leurs candidats. Le port rendait la
   première paire qui liait. Ce changement ferme `0737`.
3. **`0494`, l'indicateur de temps d'un PROJ-based.** `PROJBasedOperation::create` fixe
   `requiresPerCoordinateInputTime` d'après la chaîne PROJ écrite
   (`PROJStringFormatter::requiresPerCoordinateInputTime`, io.cpp:10529) : `helmert` ou
   `deformation` avec `+t_epoch`, `defmodel`, sauf si `+proj=set +v_4` vient avant. Les étapes
   horizontales s'annulent dans le pipeline de `EPSG:10659 → EPSG:11078` (la grille verticale
   reste seule) : PROJ n'écrit pas « time-dependent operation ». Le port interrogeait les étapes.
4. **`0275`, `std::sort` n'est pas stable.** Les candidats par ITRF2020 existent en double avant
   `removeUninterestingOps` (le pivot de datum, et l'inverse du chemin géocentrique → géographique
   dont les deux conversions s'annulent). `FilterResults::sort` appelle `std::sort`
   (coordinateoperationfactory.cpp:1552) : à égalité l'ordre est celui de l'introsort de libstdc++,
   et `removeUninterestingOps` garde le premier. L'emprise gardée est donc « World » (étendue
   synthétique de la dernière conversion), pas « World. » (celle de la base). `introsort.rs` porte
   l'algorithme (boucle d'introsort, tri par insertion final, repli en tas), vérifié contre
   `std::sort` sur 256 entrées aléatoires et adverses. Deux copies ne sont égales que si les deux
   conversions annulées le sont : une conversion géographique ⇄ géocentrique tenue pour une
   transformation basculait son drapeau `inverted`, alors que `Conversion::inverse` de PROJ rend
   une conversion neuve pour la méthode 9602.
5. **`0128`, l'autorité préférée des candidats.** `createGeodeticCRSFromDatum` reçoit l'autorité
   du CRS demandé : avec un seul identifiant, de la même autorité qu'un identifiant du datum, seuls
   les CRS de cette autorité sont candidats (factory.cpp:9990-10015). Le port lisait toutes les
   autorités et pivotait par l'ESRI `GCS_MONREF_1997` pour une paire EPSG.

## Lot Q2f — SHGD2015 et MGI, trois causes, trois listes fermées

Deux paires dont les valeurs calculées étaient déjà identiques et dont les listes de candidats
différaient : `EPSG:7885 → EPSG:9517` et `→ EPSG:7956` (24 lignes du registre réseau), et
`EPSG:11056 → EPSG:9500`. Méthode des lots précédents : libproj 9.9.0 compilé avec
`TRACE_CREATE_OPERATIONS` (seul `coordinateoperationfactory.cpp` recompilé, relié à la
bibliothèque de la référence par `LD_LIBRARY_PATH`), arbre d'appels comparé à des sondes
temporaires dans le port. Chaque cause a son test de caractérisation (valeurs de `projinfo`
9.9.0, rouge sur l'ancien code) dans `tests/compound_candidate_set.rs`.

1. **MGI : un candidat PROJ-based compte pour une étape.** `removeUninterestingOps`
   (coordinateoperationfactory.cpp:1608-1656) retire une opération de précision égale ou pire, de
   même emprise, qui a plus d'étapes de transformation que la précédente.
   `getTransformationStepCount` (1195) ne compte les étapes que d'une `ConcatenatedOperation`.
   Les candidats « horizontal + vertical + horizontal » de `createOperationsCompoundToGeog` sont
   des `PROJBasedOperation`, donc des `SingleOperation` (`createPROJBased`, 2633-2645) : une étape.
   Le port les garde sous forme concaténée marquée `proj_based` et en comptait trois, si bien que
   `Inverse of Ballpark … MGI to ETRS89-AUT [2002] + MGI to EVRF2000 Austria height (1) + {Inverse
   of Ballpark … ETRS89 to MGI | MGI to ETRS89 (8) | MGI to ETRS89 (1)}` tombait derrière le
   candidat vertical `EPSG:11503` de même emprise « Austria. ». Seul `(4)` survivait : son emprise,
   « Austria - Styria. », diffère. Le candidat par `MGI to ETRS89-AUT [2002] (8)` (le pivot par `ETRS89-AUT [2002]`)
   n'est pas dans la liste de PROJ non plus ; après la correction, les deux listes concordent.
2. **SHGD2015 : un résultat parfait du registre termine la recherche.**
   `createOperationsFromDatabase` rend `true` dès que `findOpsInRegistryDirect` donne une
   opération de précision 0 qui contient la zone (`hasPerfectAccuracyResult`, 3562-3573, appelée
   en 4351) ; `createOperations` rend alors ce résultat et n'exécute aucune autre branche
   (3652-3655). `EPSG:10614` (« SHGD2015 to SHGD2015 + SHVD2015 height (1) », précision 0,
   substituée par `us_nga_egm08_25.tif`, donc `DERIVED_FROM(EPSG)`, `singleoperation.cpp:2321`)
   est ce résultat. Le port poursuivait par la branche composé → géographique, dont le candidat
   `EPSG:7891` a la même chaîne PROJ et la même emprise : `removeDuplicateOps` gardait celui-ci et
   retirait l'opération du registre.
3. **SHGD2015, `→ EPSG:7956` : pas de ballpark vertical après un résultat parfait.**
   `createOperationsVertToGeogSynthetized` n'est appelée que par `createOperationsVertToGeog`
   (3748-3751, 6068-6108), c'est-à-dire quand `createOperationsFromDatabase` a rendu `false`. La
   jambe verticale `SHVD2015 height → SHGD2015` est `EPSG:7891` (précision 0) : PROJ n'y ajoute
   pas le changement d'unité ballpark. Le port l'ajoutait à deux endroits
   (`vertical_to_geog_candidates` pour la partie verticale d'un composé, fin de
   `create_operations`) ; les deux testent maintenant le résultat parfait.

Listes (`projinfo -s S -t T --spatial-test intersects --grid-check known_available --summary`) :

| Paire | PROJ 9.9.0 | Port avant | Port après |
|---|---|---|---|
| `EPSG:7885 → EPSG:9517` | 1 : `DERIVED_FROM(EPSG):10614` | 2 : `SHGD2015 to SHVD2015 height (1)` (sans identifiant), inverse ballpark vertical | identique à PROJ (sortie identique, lignes comprises) |
| `EPSG:7885 → EPSG:7956` | 1 : `SHGD2015 to SHVD2015 height (1) + UTM zone 30S` | 2 : la même et `Inverse of Transformation from SHVD2015 height to SHGD2015 (ballpark …) + UTM zone 30S` | identique |
| `EPSG:11056 → EPSG:9500` | 7 | 4 (manquent `Inverse of Ballpark … ETRS89 to MGI`, `MGI to ETRS89 (8)`, `(1)`) | identique, même ordre |

Volume réseau (méthode de `PORTING-cs2cs-network-only-best.md`, § « Volume réseau » : miroir local
du CDN qui journalise chaque requête, `PROJ_NETWORK_ENDPOINT`, cache utilisateur vide, les deux
binaires sur le même `PROJ_DATA`, sur 8 points par paire) :

| Paire | PROJ 9.9.0 | Port avant | Port après |
|---|---|---|---|
| `net-default-0277` (`EPSG:11056 → EPSG:9500`) | 4 requêtes, 68 495 octets | 2 requêtes, 29 948 octets (manquent les deux plages de `at_bev_GEOID_BESSEL_Oesterreich.tif`, `bytes=0-16383` et `16384-49151`) | identique : mêmes plages, même ordre |
| `net-default-0274` (`→ EPSG:7956`) | 6 requêtes, 704 512 octets | 2 requêtes, 180 224 octets | identique |
| `net-default-0275` (`→ EPSG:9517`) | 7 requêtes, 901 120 octets | 2 requêtes, 180 224 octets | identique |

Avant, la liste du port n'avait pas le candidat qui lit la grille BESSEL ; PROJ l'essaie au deuxième
point. Pour SHGD2015, le port ne demandait pas 4 (`→ EPSG:7956`) et 5 (`→ EPSG:9517`) plages de
`us_nga_egm08_25.tif` que PROJ demande.

## Reste ouvert

Grand corpus après le lot Q2e : 0 écart nouveau, les 15 lignes restantes du registre fermées
(6 listes). Le registre `tests/parity/wide/known.tsv` est vide. Après le lot Q2f : toujours 0 écart
sur le grand corpus, sur les exports (registre vide), sur le corpus avec grilles (10 276
identiques) et sur le corpus réseau (9 368 identiques, registre vide).

Limite connue : `introsort.rs` reproduit `std::sort` de libstdc++ sur toutes les plates-formes, car
les attendus viennent de PROJ compilé avec libstdc++ sous Linux. Un PROJ compilé avec libc++
(macOS) ou MSVC ordonne autrement les égalités et peut garder l'autre copie.
