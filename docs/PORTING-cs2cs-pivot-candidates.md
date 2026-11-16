# Fiche d'état — candidats pivot de `cs2cs` (lot P4)

Statut : familles A (sept cas), B, C et D fermées. Mesure (`tools/compare-cs2cs.sh --no-build`,
285 cas, deux modes) : `same 281 / tol 4 / num 0 / status 0`, avant `same 280 / tol 4 / num 1`.
Parité : 4171 identiques, 64 dans le seuil, 0 hors seuil ; `tests/parity/known.tsv` est vide.

Référence : PROJ 9.9.0, `src/iso19111/operation/coordinateoperationfactory.cpp` et
`src/iso19111/factory.cpp`. Port : `src/iso19111/factory.rs` (`datum_pivot_operations`,
`crs_codes_intermediate_operations`, `datum_based_intermediate_operations`, `filter_and_sort`),
`src/bin/cs2cs.rs`.

## Le `proj.db` n'est pas la cause

- `data/proj.db` et `target/proj-reference/build/data/proj.db` ont la même taille
  (10 551 296 octets) mais des md5 différents.
- `sqlite3 .dump` des deux fichiers : identiques à l'exception des lignes
  `sqlite_stat1` (statistiques de `ANALYZE`), qui ne diffèrent que par l'ordre.
  Le dump trié, sans `sqlite_stat1`, est identique. Mêmes `metadata`, `user_version`,
  `page_size`, mêmes nombres de lignes dans toutes les tables utiles.
- Le binaire charge le `proj.db` embarqué (`src/projdb.rs`, `include_bytes!`).

L'écart de md5 vient de l'état physique des pages, pas du contenu.

## Mesure de base

`tools/compare-cs2cs.sh --no-build` (base du lot, 275 cas d'origine) :
`same 271 / tol 4 / text 0 / num 0 / lines 0 / status 0`, respecté.

Avec les 10 cas du lot ajoutés (285 cas) : `same 271 / tol 4 / text 0 / num 10 /
lines 0 / status 0`, en défaut et avec `-1`. Les 10 cas ajoutés sont tous `num`.

## Famille A — candidat pivot gardé par le port, écarté par PROJ (fermée)

Cas : `EPSG:3701 → EPSG:32545` (17 m en est, 3 m en nord), `EPSG:32260 → EPSG:5896`,
`EPSG:22525 → EPSG:32352` avec `-I`, `EPSG:32229 → EPSG:5111`, `EPSG:21350 → EPSG:32502`,
`EPSG:32347 → EPSG:32194`, `--3d EPSG:8698 → EPSG:32427`.

Le port gardait un pivot par WGS 84 : `Inverse of Wisconsin Transverse Mercator 83 + NAD83(NSRS2007)
to WGS 84 (1) + Inverse of WGS 72BE to WGS 84 (1) + UTM zone 45S`, 4 m, étendue « Puerto Rico ...
USA » (EPSG:15931). PROJ ne gardait que le ballpark.

### Cause

Le pivot passe par une recherche de CRS intermédiaire : `WGS 72BE → NAD83(NSRS2007)` est lu par
`createFromCRSCodesWithIntermediates` (appelée par `findsOpsInRegistryWithIntermediate`,
coordinateoperationfactory.cpp:2238), et la recherche par datum (`createBetweenGeodeticCRSWithDatumBasedIntermediates`)
a le même contrôle. Ces deux recherches reçoivent `context.extent1` et `context.extent2`, les
emprises des CRS de l'appel le plus externe, héritées par tous les sous-appels (`Private::Context`,
coordinateoperationfactory.cpp:8082-8094). Chaque jambe `v1` et `v2` doit couper ces deux emprises
(factory.cpp:7845-7870, `intersects_bbox`).

Pour `EPSG:3701 → EPSG:32545`, l'emprise du second CRS est la zone UTM 45S (84°E à 90°E, 80°S à
0°). La jambe `NAD83(NSRS2007) → WGS 84 (1)` (EPSG:15931, 14,92°N à 74,71°N, 167,65°E à 63,88°O)
ne la coupe pas : PROJ l'écarte, et ne garde que le ballpark.

Le port lisait les emprises des CRS de l'appel de sous-recherche (`WGS 72BE` et `NAD83(NSRS2007)`,
toutes deux « monde » ou proches), et gardait la jambe. Le port lit désormais les emprises héritées :
`crs_extents_of(context, source, target)`, comme PROJ. Corrigé dans les deux recherches
(`crs_codes_intermediate_operations` et `datum_based_intermediate_operations`) : la correction de
la seule recherche par CRS ne suffit pas (mesuré : le pivot reste).

### Sonde (libproj 9.9.0 instrumentée, copie hors dépôt)

- Défaut (`smallest`) : `createOperationsWithDatumPivot` ne produit rien pour `WGS 72BE → NAD83`.
  `findsOpsInRegistryWithIntermediate` → `createFromCRSCodesWithIntermediates` rend 0 ligne.
- `--crs-extent-use none` : la même recherche rend 2 lignes, dont
  `WGS 72BE to WGS 84 (1) + Inverse of NAD83(NSRS2007) to WGS 84 (1)`.
- Le filtre final (`FilterResults`, emprise de la zone la plus petite, contenu strict) garderait le
  pivot s'il existait. La cause est donc en amont, dans la recherche intermédiaire.

### Mesure

`tools/compare-cs2cs.sh --no-build` (285 cas) : avant `same 271 / tol 4 / text 0 / num 10 / lines 0 /
status 0` ; après `same 278 / tol 4 / text 0 / num 3 / lines 0 / status 0`. Les 7 cas de la famille
A passent en `same`.

`cargo test --test parity` : avant `identiques 3974 · dans le seuil 68 · erreur des deux côtés 1 ·
hors seuil 187 · statut 1 · forme 6` ; après `identiques 4016 · dans le seuil 68 · erreur des deux
côtés 1 · hors seuil 145 · statut 1 · forme 6`. 42 écarts connus disparus (les 7 cas, six lignes
chacun, retirés de `known.tsv`), 0 écart nouveau.

`tests/pivot_candidate_set.rs` : deux tests (`pivot_with_a_leg_outside_the_crs_extent_is_dropped`,
`pivot_with_a_leg_outside_the_crs_extent_is_kept_without_extent_use`), valeurs de
`projinfo -s EPSG:3701 -t EPSG:32545 --summary --spatial-test intersects` (PROJ 9.9.0) et de
`--crs-extent-use none`.

### Tentative 1 (annulée)

- Changement : dans `datum_pivot_operations`, la recherche registre de chaque jambe prend les
  étendues héritées au lieu de celles de la jambe.
- Résultat : aucun effet sur `EPSG:3701 → EPSG:32545`. La recherche qui compte est la recherche
  intermédiaire, pas la recherche registre des jambes.

### Reste ouvert dans la même famille

`datum_pivot_operations` appelle `direct_registry_operations` avec les emprises des CRS candidats
(`boxes`). PROJ lit `context.extent1/2` (`createFromCoordinateReferenceSystemCodes`,
coordinateoperationfactory.cpp:1942-1953). Aucun cas mesuré ne change avec cette correction : elle
n'est pas faite.

## Famille B — candidats 3D chaînés (fermée)

Cas : `-d 12 EPSG:6310 +to EPSG:4919` (CGRS93 géographique 3D vers ITRF2000 géocentrique), et la
version 2D `EPSG:6311 → EPSG:4919`. PROJ 9.9.0 donne deux candidats à 1 m :

- `Null geographic offset from CGRS93 (geog3D) to CGRS93 (geog2D) + CGRS93 to WGS 84 (1) +
  Inverse of ITRF_2000_To_WGS_1984 + Null geographic offset from ITRF2000 to MONREF_1997_3D +
  axis order change (geographic3D horizontal) + Conversion from ITRF2000 (geographic) to ITRF2000`
- la même chaîne avec `CGRS_1993_To_WGS_1984` (ESRI:108129) à la place de EPSG:7721.

Le port ne donnait que le ballpark (2,1 m en X, 3,1 m en Y).

### Causes et correction

1. `createOperationsGeodToGeod` (`coordinateoperationfactory.cpp:5366-5387`) : pour un CRS
   géographique et un CRS géocentrique de datums différents, PROJ appelle `createOperations`
   COMPLET entre le géographique intermédiaire du géocentrique (`ITRF2000 (geographic)`, sans
   identifiant) et l'autre CRS, puis compose avec la conversion géographique/géocentrique.
   Le port n'y prenait que les opérations géodésiques simples, donc le ballpark seul.
   Correction : `CoordinateOperationFactory::geographic_geocentric_pivot`
   (`src/iso19111/factory.rs`), branché là où le registre ne donne rien. Le CRS intermédiaire
   sans identifiant suit alors la branche « source sans identifiant » de
   `findsOpsInRegistryWithIntermediate` déjà portée (`intermediate_through_candidates`) : son
   candidat `MONREF_1997_3D` (ESRI:104653, même datum EPSG:6656) ouvre les autorités
   `PROJ,ESRI,EPSG` (`authority_to_authority_preference`), d'où l'opération ESRI:108280.
   Avec deux identifiants EPSG (`EPSG:6311 → EPSG:8997`), ESRI est exclu : PROJ comme le port
   n'y donnent que le ballpark.
2. Nom de l'inverse d'un changement d'ordre des axes : PROJ recrée la conversion
   (`Conversion::inverse`, `conversion.cpp:2732-2738`, `createAxisOrderReversal`), sans
   « Inverse of ». Le port écrivait `Inverse of axis order change (geographic3D horizontal)`.
   Correction : `Conversion::inverse_by_method` (`src/iso19111/buildop.rs`), méthodes 9843 et
   9844.

La recherche par codes CRS ajoutée par la tentative 2 du lot P4c n'est pas nécessaire : retirée.

### Mesures

- `projinfo -s EPSG:6310 -t EPSG:4919 --spatial-test intersects --summary` : texte identique à
  PROJ 9.9.0 (même empreinte md5). `-o PROJ -q` identique aussi pour `6311 → 4919`,
  `4919 → 6310`, `4919 → 6311`, `6310 → 4978`, `4258 → 4919`, `4937 → 4936`, `4326 → 4978`.
- `cs2cs -d 12 EPSG:6310 +to EPSG:4919` : identique au bit sur les points testés.
- `tests/geog3d_geocentric_pivot.rs` : liste des deux candidats et valeurs `cs2cs` des six points
  du corpus (rouge avant, vert après).

### Écarts trouvés en passant, antérieurs à ce correctif (ouverts)

La liste des candidats (`projinfo --summary`) diffère de PROJ, avant comme après ce correctif
(sortie du port identique avant et après), sur : `EPSG:4979 → EPSG:7789` (61 candidats contre
62 ; PROJ ne passe pas par `Null geographic offset ... (geog3D) to ... (geog2D)` en tête),
`EPSG:7789 → EPSG:4979` (60 contre 58), `EPSG:9000 → EPSG:4978` (60 contre 58),
`EPSG:4269 → EPSG:4978` (53 contre 54), `EPSG:4167 → EPSG:4919` (même nombre, texte différent),
`EPSG:4283 → EPSG:7789` (16 contre 11). Non mesuré : l'effet sur les valeurs `cs2cs`
(le premier candidat peut être le même).

## Famille C — `--3d` ignoré à la sélection

Cas : `--3d EPSG:7680 → EPSG:2657` (110 m), `--3d EPSG:8698 → EPSG:32427` (3 m).

- `cs2cs --3d` : la sortie du port est identique à la sortie sans `--3d`
  (`-10002064.3159 23500174.0461 1061.2266`). PROJ avec `--3d` donne
  `-10002177.5276 23500104.2763 1059.5550` : hauteur inchangée, planimétrie décalée.
- `src/bin/cs2cs.rs` (`build_engine`) : avec `--3d`, les textes ne sont pas promus.
  PROJ fait `proj_crs_promote_to_3D` sur les deux CRS (`cs2cs.cpp`, branche
  `promoteTo3D`).
- `projinfo --3d` donne le même candidat que PROJ pour `EPSG:7680 → EPSG:2657`. L'écart
  est donc dans le chemin `cs2cs`, pas dans la recherche.
- Pour `EPSG:8698 → EPSG:32427 --3d`, `projinfo` du port garde le pivot `RSAO13 to WGS 84
  (1) + Inverse of WGS 72BE to WGS 84 (1) + UTM zone 27N`, 3 m ; PROJ garde le ballpark.
  C'est la famille A.

### Tentative 1 (annulée)

- Changement : `--3d` promeut chaque côté avec `promoted_text` (identifiant de la CRS
  promue).
- Résultat : sortie inchangée. La CRS 3D promue de `EPSG:7680` n'a pas d'identifiant :
  la promotion par texte ne peut rien donner.

### Correction (lot P4a)

PROJ promeut les deux CRS sans condition d'identifiant (`cs2cs.cpp:881-892`) puis
appelle `proj_create_crs_to_crs_from_pj` sur les objets. Le port fait pareil :

- `src/iso19111/select.rs` : `select_alternatives_for_crs` prend deux `Crs` et une
  `SelectionRequest` (politique, filtre, autorité, zone). `select_alternatives_in_area_with_rules`
  lit les textes puis l'appelle : même chemin pour les appels existants.
- `src/bin/cs2cs.rs` : avec `--3d`, `iso_selection` lit chaque côté en `InitRules::Proj4`,
  le promeut (`Crs::promote_to_3d`) et passe les objets. Les textes restent les noms des
  messages d'erreur. Les replis par texte (chemins « side pair », EPSG, façade) ne sont
  pas promus : c'est l'état d'avant, et PROJ n'a pas ces replis.

Mesure (référence PROJ 9.9.0, `-d 10`) :

| Cas | Avant | Après |
|---|---|---|
| `--3d EPSG:7680 +to EPSG:2657`, 6 points du corpus | premier point : `-10002064.3159` contre PROJ `-10002177.5276` | identique à la sortie PROJ (`same`) |
| `--3d EPSG:8698 +to EPSG:32427`, 6 points, premier point | x : 31,8 m de moins ; hauteur `1904.806` (juste) | x : 31,8 m de moins ; hauteur `1909.451` (4,6 m de trop) |
| `--3d EPSG:8698`, écart maximal des 6 points (corpus `compare-cs2cs`) | non relevé | x 34,3 m ; y 6,7 m ; hauteur 4,98 m |
| `--3d` 8936, 7659, 9002 (déjà justes) | identiques | identiques |

Le cas `EPSG:8698` ne passe pas. Le pivot de la famille A est retenu (voir ci-dessus). Sur
ce pivot, la hauteur est déplacée par le passage géocentrique 3D ; PROJ garde le ballpark,
qui ne touche pas la hauteur. Avant ce lot, la sortie `--3d` était la sortie 2D, donc la
hauteur était juste par hasard. Le pivot est donc à corriger avec la famille A, pas à part.

Non vérifié : les chemins de repli (`SidePair`, EPSG, façade) avec `--3d`. Les cas
`--3d` du corpus (`tools/compare-cs2cs`) et du registre de parité ne les touchent pas.

## Famille D — pivot par WGS 84 que le port ne construisait pas (fermée)

Cas : `EPSG:26898 → EPSG:26957` (six lignes de `known.tsv`, 1,24 m en est).

PROJ garde deux pivots par WGS 84, 5 m et 6 m :
`Inverse of MTM zone 1 + NAD83(CSRS) to WGS 84 (2) + Inverse of NAD83 to WGS 84 (1) + SPCS83
Delaware zone (meter)` et la même avec `(1)`. Le port ne gardait que le ballpark.

Ce cas ne vient pas de la famille A. Le port d'avant `572fc58` donnait déjà le ballpark seul
(`projinfo` d'avant A, même sortie en `--crs-extent-use both`).

### Cause

Sonde (libproj 9.9.0 instrumentée, copie hors dépôt, appels de `findsOpsInRegistryWithIntermediate`
étiquetés) :

- Le registre direct `NAD83 → NAD83(CSRS)` rend un op : `NAD83 to NAD83(CSRS) (4)`, 2 m, emprise
  « Canada offshore ». Les emprises de l'appel externe (Terre-Neuve, Delaware) l'écartent : après
  filtre, `res` est vide. Avant filtre, le résultat n'est pas vide (`resFindDirectNonEmptyBeforeFiltering`).
- `createOperationsFromDatabase` ouvre alors la porte des pivots intermédiaires
  (coordinateoperationfactory.cpp:4472-4474) : `useOnlyReplacedOperations(res)` est vrai sur une
  liste vide. `findsOpsInRegistryWithIntermediate` rend les deux pivots (appel de la porte, cpp:4503,
  `INTERMEDIATE raw=2`).
- Le port gardait l'op de registre non filtré dans `found`. Sa porte exige `!direct_unfiltered_empty`
  et lit `found`, qui contient le 2 m : porte fermée, aucun pivot.
- Même défaut dans la passe par datum : `create_candidates` rend le 2 m non filtré, la jambe
  `NAD83 → NAD83(CSRS)` le relie à des jambes nulles, et ce pivot ferme la porte. PROJ ne le
  rend pas : `createOperations` appelle `findOpsInRegistryDirect`, qui filtre (cpp:1953).

### Correction (lot P4d)

`create_candidates_at` (`src/iso19111/factory.rs`) : le résultat du registre direct est filtré par
`filter_and_sort` avant d'entrer dans `found`, comme `findOpsInRegistryDirect`. Le drapeau
`direct_unfiltered_empty` garde la valeur d'avant filtre.

Mesure :

- `tests/pivot_candidate_set.rs` : `mtm1_to_delaware_keeps_both_wgs84_pivots` échoue avant la
  correction (le port rend le ballpark seul) et passe après (liste de `projinfo`).
  `mtm1_to_delaware_without_extent_use_keeps_the_direct_operation` passe avant et après.
- `tools/compare-cs2cs.sh --no-build` : seul le cas 26898 change, `num → same`.
- `cargo test --test parity` : avant `identiques 4159 · hors seuil 12`, après `identiques 4165 ·
  hors seuil 6`. Six écarts connus disparus, retirés de `known.tsv`, zéro nouveau.

### Non mesuré (reste de la famille D)

- Les autres points où le port lit `found` sans filtre : `has_too_small_areas` et le test
  `res.is_empty()` de la porte. PROJ les lit sur `res` filtré. Identique pour ce cas, non mesuré ailleurs.
- Le drapeau : PROJ ne le lève que pour les autres autorités que `PROJ` (`if (authName == "PROJ")
  continue;`, cpp:1944). Le port ne fait pas cette distinction.
- La conversion de projection poussée dans `found` avant le registre (`projection_uses_source`) :
  PROJ ne la met pas dans `res`.
- `datum_pivot_operations` appelle `direct_registry_operations` avec les emprises des CRS candidats
  (`boxes`) ; PROJ lit `context.extent1/2`. Voir « Reste ouvert dans la même famille » (A).

## Écart trouvé hors registre (non mesuré, non corrigé)

`precedes` (`src/iso19111/select.rs`, branche ETRS89) contient une règle que PROJ 9.9.0
n'a pas :

- Port : `b.uses_etrs89_generic && !b.uses_etrf2000 && a.uses_etrf2000 &&
  !a.uses_etrs89_generic` renvoie vrai.
- PROJ (`coordinateoperationfactory.cpp:1037-1046`) : la première condition contient
  `iterA.uses_ETRF2000_ && !iterA.uses_ETRF2000_`, donc elle ne renvoie jamais vrai.

Le port diverge de PROJ 9.9.0 sur ce point. Aucun cas du registre ni du corpus de
parité ne l'a mesuré. À traiter dans un lot séparé, après mesure.

## Non vérifié

- Les familles A, B, C et D ne sont pas vérifiées sur d'autres paires que celles listées.
