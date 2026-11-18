# Création : ne pas relire deux fois la même ligne

Ce lot ne porte pas de code PROJ. Il mesure ce que la création d'une transformation relit dans
`proj.db` et ne le relit plus pendant une recherche. Aucun résultat ne change : mêmes candidats,
même ordre, mêmes textes exportés (WKT, chaînes PROJ, JSON).

## Ce qui était reconstruit

`Proj::new_known_crs`, `proj_create_crs_to_crs` et `create_operations*` lisent des CRS, des datums,
des ellipsoïdes, des méridiens d'origine, des systèmes de coordonnées, des unités et des étendues, et
construisent les objets ISO 19111. Une recherche demande les mêmes lignes à la base des dizaines de
fois : un ensemble de datums (EPSG:4326 en a huit) relit l'ellipsoïde, le méridien et l'unité de
chaque membre, et chaque candidat relit les domaines de ses CRS.

Mesure : compteurs temporaires (non livrés) dans `DatabaseContext::query` et dans les `create_*`
d'`AuthorityFactory`, binaire `--release`, une création par paire sur un contexte neuf. Appels /
appels distincts (même autorité et même code) :

| objet construit | `4326 → 2154` | `4267 → 4326` (80 candidats) |
|---|---:|---:|
| `properties` (identifiants, domaines) | 133 / 26 | 297 / 152 |
| `create_unit_of_measure` | 129 / 3 | 154 / 6 |
| `create_prime_meridian` | 35 / 1 | 36 / 1 |
| `create_ellipsoid` | 35 / 2 | 36 / 3 |
| `create_coordinate_system` | 23 / 2 | 22 / 1 |
| `create_datum` | 16 / 8 | 16 / 8 |
| `create_geodetic_crs` | 21 / 10 | 22 / 12 |
| `create_coordinate_reference_system` | 13 / 11 | 14 / 12 |
| `create_shared_crs` (déjà en cache) | 20 / 11 | 350 / 12 |
| requêtes SQL (texte et paramètres) | 642 / 148 | 2 141 / 678 |

Sur 30 paires (`4326 → 2154`, `4267 → 4326`, `4230 → 4326`, `27700 → 2154`, `4277 → 4258`,
`7405 → 4979`, `3035 → 4326`, `31370 → 4326`… la liste est dans la section « Mesures ») :
35 796 requêtes, 6 621 distinctes, **81 % redondantes**. Le temps passé dans `query` est de 612 ms
sur les 780 ms environ de la création des 30 paires ; en comptant chaque rappel d'une même requête au
temps moyen de ses appels, 458 ms sont redondants. Les requêtes qui pèsent le plus :

| requête (début du texte) | appels | distinctes | ms | dont redondants |
|---|---:|---:|---:|---:|
| `SELECT auth_name, code FROM alias_name WHERE table_name = ? AND alt_name = ? AND source IN …` | 236 | 41 | 189,0 | 157,7 |
| `SELECT table_name, … is_alias FROM (SELECT 'geodetic_crs:geographic …` (recherche par nom) | 162 | 15 | 116,1 | 105,3 |
| `SELECT auth_name, code FROM "geodetic_datum" WHERE name = ? ORDER BY deprecated` | 1 228 | 211 | 65,9 | 55,2 |
| `SELECT table_name, … is_alias FROM (SELECT 'geodetic_datum' …` | 60 | 15 | 60,3 | 45,2 |
| `SELECT extent.description, … FROM usage JOIN extent JOIN scope …` (domaines) | 8 605 | 1 518 | 20,2 | 14,5 |
| `SELECT axis.name, abbrev, orientation … FROM axis …` | 632 | 68 | 15,9 | 13,9 |
| `SELECT cov.source_crs_auth_name, … ` (candidats) | 991 | 776 | 42,6 | 13,0 |
| `SELECT alt_name FROM alias_name WHERE table_name = ? AND auth_name = ? AND code = ?` | 2 160 | 196 | 9,0 | 8,1 |
| `SELECT name, conv_factor, type FROM unit_of_measure …` | 6 809 | 129 | 7,1 | 6,9 |
| `SELECT ellipsoid.name, … FROM ellipsoid …` | 2 481 | 60 | 5,0 | 4,6 |

Les gros postes ne sont pas des objets construits mais des recherches par nom (comparaison de
datums et de CRS, `REPLACE(LOWER(name), …)` sans index) rappelées avec les mêmes paramètres.

## Décision : garder les lignes, pas les objets

Un cache par type d'objet (unité, ellipsoïde, méridien, système de coordonnées, datum, domaines)
n'aurait pas touché les recherches par nom, qui font près de 80 % du temps redondant. Un cache des
lignes de `DatabaseContext::query` couvre tout d'un coup, sans règle par type. Ce qu'il laisse :
reconstruire les objets à partir des lignes gardées. Mesuré (callgrind, `4267 → 4326`, avec le cache) :
`create_coordinate_reference_system` 1,8 % des instructions, `properties` 1,8 %, `usages` 1,7 %. Un
cache d'objets par-dessus gagnerait de l'ordre de 5 % au mieux et clonerait des objets à chaque
lecture. Non fait.

## Ce qui est gardé

`SearchCache` (`src/iso19111/search_cache.rs`) prolonge le mécanisme existant : il remplace
`Connections::crs_cache` dans la connexion d'un `DatabaseContext` et en garde la discipline.

- **Lignes** : clef = texte SQL et paramètres, valeur = lignes lues. Une requête qui rend plus de
  64 lignes n'est pas gardée (la recopier coûte plus que la relire). Plafond de 512 KiB comptés :
  au-delà, plus d'insertion, les lectures suivantes vont à la base. Le plus gros relevé sur les
  30 paires (`4267 → 4326`) compte 266 KiB.
- **CRS partagés** : ceux de `create_shared_crs` (64 entrées, clef `(autorité, code)`), comme avant.
- **Durée de vie** : le cache ne retient rien tant qu'aucune recherche ne tourne sur le contexte.
  `SearchScope` (qui remplace `FlushStatements`) ouvre la recherche (`create_operations_between_epochs`)
  et la ferme : la dernière recherche du contexte vide le cache, et la plus externe de son fil vide
  aussi les requêtes compilées, comme avant. Les trois entrées publiques qui construisent des
  opérations hors recherche (`create_coordinate_operation`,
  `create_from_coordinate_reference_system_codes(_in)`, `create_from_crs_codes_with_intermediates`)
  ouvrent un `SearchScope::lookups` : le cache sert pendant l'appel, les requêtes compilées restent.
- **Écriture** : `execute_sql` et une session d'insertion vident le cache avant et après
  l'écriture. Un jeton de génération empêche une lecture commencée avant l'écriture d'insérer ses
  lignes après : un autre fil qui lit le même contexte ne peut pas y laisser une ligne d'avant.
- **Ordre** : la portée s'ouvre avant `resolve_crs`. Sinon le CRS que la recherche remplace par
  celui du cache n'est pas celui que les candidats retrouvent, et `4267 → 4326` n'a plus un seul
  objet source mais deux (`tests/shared_crs.rs` l'a attrapé).
- **Pas gardé** : une erreur ; les trois requêtes de `network.rs` (noms de grilles), qui passent par
  `projdb::with_db` et non par `query` (556 appels, 1,8 ms pour `4267 → 4326`).

Écart avec le lot précédent : le cache de CRS durait aussi longtemps que le contexte (jusqu'à une
écriture) ; il s'arrête maintenant avec la recherche. Un contexte que l'appelant garde ne retient
plus 64 CRS au repos, et relit ses CRS à la recherche suivante.

## Garde-fous

`src/iso19111/search_cache/tests.rs` :

- rien n'est gardé hors recherche ; un cache rempli par deux portées imbriquées ne se vide qu'à la
  sortie de la dernière ;
- une recherche (`4267 → 4326`, plus de 100 lectures servies par le cache) laisse le contexte vide,
  puis une seconde (`4326 → 2154`) aussi ;
- une écriture (`execute_sql`) vide le cache, et la lecture suivante voit l'écriture ;
- une lecture dépassée par une écriture n'est pas gardée ; un résultat trop long et un budget plein
  sont laissés de côté ;
- **mêmes résultats avec et sans le cache** : six paires (`4267 → 4326`, `4326 → 2154`,
  `2154 → 32631`, `4277 → 4258`, `27700 → 2154`, `7405 → 4979`), la liste complète des candidats
  comparée en `Debug` (modèle entier : CRS, paramètres, domaines, grilles), le cache coupé pour un
  fil par un crochet de test.

Les corpus de parité (`parity`, `parity_wide`, `parity_export`, `parity_grids`), les tests de la
ligne de commande et de l'API C passent inchangés.

## Mesures

Machine cloud 4 cœurs (Intel Xeon 2,10 GHz, AVX-512), Linux, rustc 1.100, binaires `--release`
(profil du dépôt). Le programme de mesure appelle `Proj::new_known_crs` ; un processus par série,
épinglé (`taskset -c 2`), processus de l'état de départ (`a8580cd`) et du changement alternés.

### Temps de création

Médiane de 100 créations par processus, médiane de 9 processus par côté. Les cinq paires de
`FACADE_PAIRS` (`src/bin/bench.rs`) :

| paire | avant | après | écart |
|---|---:|---:|---:|
| `4326 → 2154` | 10,68 ms | **8,27** | -22,6 % |
| `4326 → 32631` | 5,92 | 5,72 | -3,3 % |
| `4326 → 3857` | 5,70 | 5,83 | +2,3 % |
| `2154 → 32631` | 11,26 | **8,87** | -21,2 % |
| `4267 → 4326` | 50,65 | **41,90** | -17,3 % |
| **somme** | 84,92 | **68,84** | **-18,9 %** |

Les deux paires à 5,7 ms n'ont presque rien de redondant (350 requêtes, 49 distinctes) : leur écart
est du bruit. Mesure dédiée de quatre paires de ce genre (`4326 → 32631`, `4326 → 3857`,
`5070 → 4269`, `3395 → 4326`), 300 créations par processus, 13 processus par côté : -0,2 %, +0,8 %,
+1,7 %, -0,7 %, somme +1,0 %. Le cache ne coûte rien de mesurable quand il n'a rien à servir.

Trente paires, 40 créations par processus, 7 processus par côté :

| paire | avant (ms) | après | écart | paire | avant (ms) | après | écart |
|---|---:|---:|---:|---|---:|---:|---:|
| `4326 → 2154` | 11,49 | 9,03 | -21,4 % | `25832 → 4326` | 13,24 | 11,18 | -15,6 % |
| `4326 → 32631` | 6,36 | 6,38 | +0,4 % | `25832 → 2154` | 32,97 | 21,37 | -35,2 % |
| `4326 → 3857` | 6,30 | 6,39 | +1,5 % | `26918 → 4326` | 31,41 | 18,95 | -39,7 % |
| `2154 → 32631` | 12,10 | 9,83 | -18,8 % | `26918 → 32618` | 35,95 | 21,67 | -39,7 % |
| `4267 → 4326` | 50,05 | 42,77 | -14,5 % | `3395 → 4326` | 6,15 | 6,01 | -2,2 % |
| `4269 → 4326` | 35,44 | 28,05 | -20,8 % | `5070 → 4269` | 5,48 | 5,99 | +9,4 % |
| `4230 → 4326` | 27,58 | 20,90 | -24,2 % | `2056 → 4326` | 10,63 | 8,26 | -22,3 % |
| `4258 → 4326` | 9,76 | 8,42 | -13,8 % | `21781 → 2056` | 30,04 | 20,15 | -32,9 % |
| `4313 → 4326` | 22,91 | 14,38 | -37,3 % | `31370 → 4326` | 28,35 | 15,76 | -44,4 % |
| `4807 → 4326` | 11,26 | 8,48 | -24,7 % | `28992 → 4326` | 11,32 | 8,37 | -26,1 % |
| `27700 → 4326` | 29,68 | 17,30 | -41,7 % | `2180 → 4326` | 13,13 | 10,17 | -22,6 % |
| `27700 → 2154` | 33,83 | 21,56 | -36,3 % | `3003 → 4326` | 28,48 | 15,95 | -44,0 % |
| `4277 → 4258` | 75,97 | 35,04 | -53,9 % | `4979 → 4326` | 5,99 | 5,39 | -10,0 % |
| `3035 → 4326` | 32,69 | 20,29 | -37,9 % | `4978 → 4326` | 5,95 | 5,41 | -9,1 % |
| `2193 → 4326` | 11,33 | 8,49 | -25,0 % | `7405 → 4979` | 147,83 | 65,34 | -55,8 % |
| | | | | **somme** | **774,3** | **494,1** | **-36,2 %** |

La ligne `5070 → 4269` (+9,4 %) est du bruit : la même paire mesurée à part donne +1,7 %.

Instructions (callgrind, une création par paire) :

| paire | avant | après | écart |
|---|---:|---:|---:|
| `4326 → 2154` | 116,5 M | 82,8 M | -28,9 % |
| `4326 → 32631` | 56,6 M | 55,5 M | -1,9 % |
| `4326 → 3857` | 56,5 M | 55,4 M | -1,9 % |
| `2154 → 32631` | 122,7 M | 88,0 M | -28,3 % |
| `4267 → 4326` | 633,3 M | 511,7 M | -19,2 % |
| 30 paires | 8 530 M | 4 951 M | -42,0 % |

### Mémoire

Pic `VmHWM`, médiane de 5 passes, un processus par scénario, programmes de `bench/footprint/rs`
(construits à la main, `bench/footprint.sh` n'est pas relancé : il réécrit `README.md` et
`bench/footprint.json`). PROJ 9.9.0 : le programme C de `bench/footprint/c`, mesuré sur la même
machine (les chiffres de `bench/footprint.json`, 5,6 / 14,7 / 14,8 / 29,8 / 16,3 MiB, sont d'une
autre machine et du même ordre). Médiane de 3 séries :

| scénario | PROJ 9.9.0 (ici) | avant | après |
|---|---:|---:|---:|
| démarrage | 5,3 MiB | 3,04 | 3,03 |
| création `4326 → 2154` | 14,3 | 11,56 | **11,71** |
| création + 1 M points un par un | 14,4 | 11,64 | **11,78** |
| création + 1 M points en tableau | 29,5 | 26,97 | **27,04** |
| `4267 → 4326` (80 candidats) + 1 M points | 15,9 | 12,57 | **12,77** |

Aucun scénario ne passe au-dessus de PROJ : le cache coûte de 0,07 à 0,20 MiB, et il reste au
moins 2,5 MiB sous PROJ (3,1 MiB pour `4267 → 4326`).

Hors des cinq scénarios, trente paires, un processus par paire (médiane de 5) : le pic monte de
+0,4 MiB au plus (`7405 → 4979`, 17,59 → 17,97 MiB), de +0,08 MiB en moyenne, et baisse de 4 à
180 KiB sur sept paires (bruit). Les 30 paires dans un seul processus : 17,93 → 18,35 MiB (PROJ :
17,92). Deux paires sont au niveau de PROJ ou au-dessus : `25832 → 2154` (avant 14,84, après 14,96, PROJ 14,90 MiB) et
`7405 → 4979` (avant 17,59, après 17,97, PROJ 15,91 MiB ; l'état de départ y était déjà au-dessus
de 1,7 MiB). Ce ne sont pas des scénarios de `bench/footprint.json`.

Tas (massif, pic tas + surcoût de l'allocateur) : `4326 → 2154` 3,84 → 3,94 MiB, `4267 → 4326`
4,65 → 4,81, `4277 → 4258` 6,17 → 6,42. Le cache pèse, en tas réel, deux à trois fois ce qu'il
compte (tables, un `String` par cellule).

## Décision

Livré. Critères : création plus rapide de 10 % ou plus sur l'ensemble mesuré (-18,9 % sur les cinq
paires de la façade, -36,2 % sur trente paires), pic de mémoire sous PROJ dans les cinq scénarios de
`bench/footprint.json`. Hors de ces scénarios, le pic monte de 0,4 MiB au plus (voir « Mémoire »).

## Ce qui reste, chiffres à l'appui

- **L'export des candidats** : après le cache, `4267 → 4326` passe 46 % de ses instructions dans
  `export_to_proj_string` → `epsg_method_code` → `method_by_name` → `canonical` (callgrind). Ce
  n'est pas de la lecture de base : un lot à part.
- **Les recherches par nom** : 211 lectures distinctes de `geodetic_datum WHERE name = ?` (une
  comparaison sans index, environ 55 µs chacune) restent, une par nom.
- **Les noms de grilles** de `network.rs` ne passent pas par le cache (voir plus haut).
