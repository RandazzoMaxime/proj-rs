# Fiche d'état — candidats pivot de `cs2cs` (lot P4)

Statut : ouvert. Les trois familles d'écarts de `tests/parity/known.tsv` restent
ouvertes. Une tentative sur la famille A a échoué ; elle est annulée. Une tentative
sur la famille C a échoué ; elle est annulée. Les cas sont dans
`tools/compare-cs2cs/cases.txt` (10 cas, tous en `num`).

Référence : PROJ 9.9.0, `src/iso19111/operation/coordinateoperationfactory.cpp`.
Port : `src/iso19111/factory.rs` (`datum_pivot_operations`,
`crs_codes_intermediate_operations`, `filter_and_sort`), `src/bin/cs2cs.rs`.

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

## Famille A — candidat pivot gardé par le port, écarté par PROJ

Cas : `EPSG:3701 → EPSG:32545` (17 m en est, 3 m en nord), `EPSG:32260 → EPSG:5896`,
`EPSG:22525 → EPSG:32352` avec `-I`.

Le port garde un candidat par pivot WGS 84 :
`Inverse of Wisconsin Transverse Mercator 83 + NAD83(NSRS2007) to WGS 84 (1) +
Inverse of WGS 72BE to WGS 84 (1) + UTM zone 45S`, 4 m, étendue « Puerto Rico ... USA ».
PROJ ne garde que le ballpark `Ballpark geographic offset from NAD83(NSRS2007) to
WGS 72BE`.

Mesures avec `projinfo -s EPSG:3701 -t EPSG:32545 -o PROJ --summary` :

| Option | PROJ 9.9.0 | Port |
|---|---|---|
| `--crs-extent-use none` | pivot 4 m + ballpark | pivot 4 m + ballpark |
| `--crs-extent-use both` | ballpark | ballpark |
| défaut (`smallest`) | ballpark | pivot 4 m |
| `--bbox -100,30,-80,50` | ballpark | pivot 4 m |
| `--bbox 84,-80,90,0` | ballpark | ballpark |
| `--pivot-crs never` | ballpark | pivot 4 m |

Le pivot vient donc de la recherche datum-pivot (`datum_pivot_operations`), pas de la
recherche par CRS intermédiaire. Dès qu'une étendue de CRS est active, PROJ retire le
pivot. Une `--bbox` utilisateur ne suffit pas à le retirer : l'étendue des CRS reste
active, et c'est elle qui compte.

Pour `EPSG:22525 → EPSG:32352` (`-I`), le port a 8 candidats pivot ; PROJ n'a que le
ballpark. Pour `EPSG:32260 → EPSG:5896`, le port en a 2 ; PROJ n'a que le ballpark.

### Tentative 1 (annulée)

- Changement : dans `datum_pivot_operations`, le lookup registre de chaque jambe prend
  les étendues héritées (`crs_extents_of`) au lieu des étendues des CRS de la jambe.
- Motif : `createTransformations` appelle `findOpsInRegistryDirect` avec
  `context.extent1` et `context.extent2` (coordinateoperationfactory.cpp:3278-3340 et
  1930-1960).
- Résultat : aucun effet sur `EPSG:3701 → EPSG:32545`, toujours un pivot.
- Annulée (`git checkout`), car sans gain démontré.

### Ce qui reste à établir

Le code de PROJ qui retire le pivot n'est pas identifié. Les lectures de
`FilterResults`, du lookup registre (préférence « intersecting », `factory.cpp:7440-7490`)
et de `createOperationsWithDatumPivot` ne le montrent pas. La bibliothèque de référence
n'a pas de trace utilisable (`PROJ_DEBUG` ne journalise rien dans les candidats).

Prochain pas suggéré : instrumenter le port (sortie temporaire des candidats de
`datum_pivot_operations` par jambe), puis comparer aux résultats intermédiaires de
PROJ obtenus par `projinfo` avec les options d'étendue ci-dessus.

## Famille B — candidat absent du port : pivot 3D

Cas : `EPSG:6310 → EPSG:4919` (`-d 12`, entrée 3D).

PROJ donne deux candidats, chaînés par `Null geographic offset from CGRS93 (geog3D) to
CGRS93 (geog2D)` puis `CGRS93 to WGS 84 (1)` et `Inverse of ITRF_2000_To_WGS_1984`.
Le port donne un seul ballpark. Le port ne construit pas la chaîne 3D. Non tenté.

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
- Résultat : sortie inchangée. La CRS 3D promue de `EPSG:7680` n'a sans doute pas
  d'identifiant, donc aucune promotion n'a eu lieu.
- Annulée.

Prochain pas : `cs2cs` doit construire depuis l'objet CRS promu (sans passer par un
texte), ce qui demande une entrée de `build_engine` qui accepte un `Crs`.

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

- Les autres paires des 10 cas (`EPSG:32229 → EPSG:5111`, `EPSG:26898 → EPSG:26957`,
  `EPSG:21350 → EPSG:32502`, `EPSG:32347 → EPSG:32194`) : pas de diagnostic. Elles
  restent `num`.
- La famille A n'est vérifiée que sur 3 paires.
