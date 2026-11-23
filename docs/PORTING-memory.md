# Mémoire : où elle va, ce qui a changé, ce qui reste

Ce lot ne porte pas de code PROJ. Il mesure le pic de mémoire résidente (`VmHWM`) de proj-rs et
retire les copies que le code faisait sans nécessité. Le contrôle des résultats est inchangé : aucun
chiffre, aucune erreur, aucun ordre de candidats ne bouge.

## Mesure

`bench/footprint.sh` et les programmes de `bench/footprint/` (le même programme en Rust et en C
contre libproj). Linux x86-64, médiane de 5 passes, pic `VmHWM`, un processus par scénario.

| scénario | PROJ 9.9.0 | avant | après #228 | après la VFS | après le partage et la conversion sur place |
|---|---:|---:|---:|---:|---:|
| démarrage | 5,6 MiB | 4,6 | 4,8 | 4,7 | 4,7 |
| création `4326 → 2154` | 14,7 | 34,3 | 18,6 | 13,9 | **13,8** |
| création + 1 M points un par un | 14,8 | 34,5 | 18,6 | 13,9 | **13,6** |
| création + 1 M points en un tableau | 29,8 | 95,4 | 49,3 | 44,2 | **29,3** |
| `4267 → 4326` (80 candidats) + 1 M points | 16,3 | 51,3 | 33,9 | 28,0 | **14,5** |

Les colonnes « avant » et « après #228 » sont celles du lot précédent. La dernière a été mesurée
avec le même programme, sur la même machine, juste après une remesure de l'état de départ
(4,7 / 18,4 / 18,6 / 48,9 / 33,6). La colonne « après la VFS » est l'état de départ du lot de
partage (`claude/memory-vfs`), remesuré ; la dernière est ce lot. Sous Linux, la création d'une paire
de CRS et le scénario à 80 candidats passent sous PROJ. Le scénario à un million de points en
tableau reste au-dessus : sa cause est en fin de page.

## Où allaient les octets

massif (tas), `smaps` (pages de fichier) et dhat sur chaque scénario.

1. **Une copie de `proj.db` par `DatabaseContext::create()`.** La base embarquée fait 10 MiB. Chaque
   contexte la recopiait dans une base mémoire modifiable (`Backup`), qui lisait aussi toutes les
   pages de la base dans le binaire : 10 MiB de tas plus 10 MiB de pages de fichier. Un contexte
   n'écrit presque jamais (une session d'insertion, `execute_sql`). La création d'une paire de CRS en
   ouvre plusieurs, imbriqués.
2. **Un tableau converti = 3 copies.** `convert_array` construisait un `Vec<Xyzt>` de toute la
   tranche (32 octets par point, soit 31 MiB pour 1 M de points), puis un `Vec<C>` des résultats
   (15 MiB), pour garantir qu'une erreur laisse la tranche intacte. Décision de l'opérateur :
   cette garantie est abandonnée.
3. **Les candidats.** Pour `4267 → 4326`, 80 opérations vivent à la fois. Trois étapes en gardaient
   une copie à côté de l'original : le filtre (`filter_and_sort`), `hasPerfectAccuracyResult`
   (qui clonait toute la liste) et `removeUninterestingOps`, plus la concaténation registre et pivot.
4. **Le schéma de SQLite.** Chaque connexion analyse le schéma de `proj.db` (2,4 MiB de tas). La
   base globale de `projdb::with_db` en ajoutait une de plus, jamais fermée.

## Ce qui a changé

- `DatabaseContext::create()` lit l'image embarquée **sur place** (`deserialize_bytes` en lecture
  seule, `mmap_size` pour que SQLite ne copie pas chaque page dans son cache). Une écriture
  (`execute_sql`, session d'insertion) prend alors une copie modifiable propre au contexte et à ses
  clones : un autre contexte ne voit pas l'écriture (`tests/shared_database.rs`).
- Les contextes d'un même fil se partagent une seule connexion en lecture, tant que l'un d'eux la
  tient (`Weak` par fil : la dernière référence la ferme, rien ne reste après la création).
  `projdb::with_db` passe par la même connexion. Un fil n'attend jamais un autre fil.
  Avec `PROJ_AUX_DB`, les lignes auxiliaires sont copiées dans la base principale : chaque contexte
  garde sa copie, comme avant.
- `convert_array`, `project_array` et `Transformer::forward_slice` / `inverse_slice` convertissent
  **sur place**, par blocs de 4096 points : un seul bloc de `Xyzt` (128 Kio) vit, et le résultat de
  chaque bloc est réécrit dans la tranche de l'appelant. Aucune copie de la taille des données :
  31 MiB de moins que l'état de départ, 15 MiB de moins que le lot précédent, pour 1 M de points.
  C'est ce que fait `proj_trans_array`. Les valeurs ne changent pas d'un bit (`tests/convert_array_golden.rs`,
  empreintes prises avant le changement : 5 paires, les deux sens, `f32`, UTM, les deux backends,
  80 candidats), et l'ordre des erreurs est celui d'avant : coordonnée d'entrée illisible, puis
  erreur de transformation du premier point en échec, puis rétrécissement vers le type de la tranche.

### Contrat d'erreur des tranches

| cas | état de la tranche | erreur rendue |
|---|---|---|
| tranche vide, ou objet qui n'est pas une transformation | intacte | `Ok` / `NotATransform` |
| coordonnée d'entrée refusée (`Transformer` : non finie ; `Proj` : illisible comme `f64`) | **intacte** : on vérifie toute la tranche d'abord, sans rien écrire | l'erreur de l'entrée |
| un point sans image | blocs **avant** le premier bloc fautif : convertis ; ce bloc et les suivants : leur entrée | l'erreur de `convert` pour le premier point en échec |
| valeur qui ne tient pas dans le type de la tranche | ce point garde son entrée ; les autres sont convertis | l'erreur de rétrécissement, après tous les blocs et après toute erreur de transformation |

Un bloc fautif n'est pas écrit : on n'y marque pas les points en échec. Le premier point en échec
et tous les suivants gardent donc leur entrée, et l'appelant ne peut pas lire la tranche comme un
résultat après un `Err`. La taille du bloc (4096) n'est pas promise.

Ce que fait PROJ : `proj_trans_array` traite **tous** les points, met `HUGE_VAL` aux quatre
composantes d'un point en échec et rend le code d'erreur commun, ou `PROJ_ERR_COORD_TRANSFM` si les
codes diffèrent. La capi (`capi/src/trans.rs`) et `Operation::transform_array` (`src/gie_api.rs`)
font déjà cela, en place ; ils n'ont pas changé. La façade s'arrête au bloc fautif au lieu de
continuer : une tranche qui échoue tôt ne télécharge pas de grille et ne choisit pas de candidat
pour chacun des points suivants, et `HUGE_VAL` ne serait pas représentable dans tout type de
coordonnée. Le crate georust `proj` 0.31 ne documente aucun contrat de la tranche après une erreur
(sa mise en œuvre copie la tranche en `PJ_COORD`, la remplit seulement si `proj_trans_array` réussit) :
la façade ne promet donc rien de moins que lui sur le papier.

- `filter_and_sort` déplace les candidats retenus au lieu de les cloner (un indice par candidat,
  un clone seulement pour un candidat retenu deux fois) ; `has_perfect_accuracy_result` compte sur
  les indices sans toucher la liste ; `removeUninterestingOps` marque puis déplace ; la liste `res` du
  pivot prend `found` et le rend, sans copie.

Vitesse : création de `4326 → 2154` de 58 à 52 ms par processus (plus de copie de 10 MiB). Les cas
`facade_convert*` du banc ne bougent pas : voir le tableau du lot.

## Pages de la base embarquée (Linux)

Le lot précédent lisait `proj.db` (10 MiB) en place dans le binaire. Les pages touchées par une
requête deviennent résidentes dans le processus (`RssFile`), et le noyau en mappe 16 d'un coup à
chaque faute de page. PROJ lit sa base avec `read` : ses pages restent dans le cache du noyau et ne
comptent pas dans son `VmHWM`.

`src/projdb_exe.rs` fait de même sous Linux. Une VFS SQLite en lecture seule répond à `xRead` par un
`pread` sur le fichier de l'exécutable, à l'offset des octets embarqués (trouvé dans
`/proc/self/maps`) : le mappage n'est jamais touché. Le format ne change pas, la base reste dans le
binaire, et la connexion est ouverte `immutable=1`. Le repli est l'ancien chemin (octets lus en
place) quand une vérification échoue : pas de `/proc`, binaire remplacé sur le disque ou supprimé
(l'inode et la première et la dernière page de l'image sont comparés), image sur plusieurs
mappages, octets qui ne viennent pas d'un fichier. Les autres systèmes gardent l'ancien chemin.

Le cache de pages de chaque connexion passe de 2000 KiB (défaut SQLite) à 500 KiB
(`CACHE_KIB`) : une création lit environ 2 MiB de pages une seule fois.

Mesures de `Proj::new_known_crs("EPSG:4326", "EPSG:2154")`, pic `VmHWM`, médiane de 5 :

| variante | création | 4267 → 4326 + 1 M points |
|---|---:|---:|
| lecture en place, état de départ | 18,4 MiB | 33,6 |
| VFS `pread`, cache 2000 KiB | 15,4 | 29,6 |
| VFS `pread`, cache 1000 KiB | 14,6 | |
| **VFS `pread`, cache 500 KiB (livré)** | **13,9** | 28,0 |
| VFS `pread`, cache 250 KiB | 13,7 | 27,9 |
| VFS `pread`, cache 100 KiB | 13,4 | 27,6 |
| VFS qui copie depuis les octets embarqués (option a) | 20,3 (cache 2000), 18,6 (cache 250) | 35,6 / 33,8 |

Un cache de 250 KiB ou moins gagne peu de plus et coûte du temps sur les requêtes qui reviennent
sur les mêmes pages (80 candidats).

### Options évaluées et rejetées

- **a. VFS dont `xRead` copie depuis les octets embarqués** (mesurée) : plus lourde que l'état de
  départ. `xRead` touche les pages du binaire, qui restent résidentes comme avant, et le cache de
  SQLite ajoute sa copie dans le tas (20,3 MiB, 18,6 avec un petit cache). Ce qui compte n'est pas
  qui copie, mais que les pages mappées ne soient jamais touchées.
- **b. Lire un fichier `proj.db` à côté de l'exécutable ou dans `PROJ_DATA`**, base embarquée en
  repli : donnerait le même résultat que la VFS sur l'exécutable (même `pread`, même cache), au prix
  d'un fichier à installer et d'une base qui peut ne pas être celle du binaire. Rien de plus pour
  la mémoire, et le produit perd « aucun fichier à livrer ». Non implémentée ; pas de fonctionnalité
  `embedded-db`.
- **c. Base compressée, décompressée à la demande par pages** : pas de dépendance de compression
  utilisable (`tiff` n'embarque que `deflate`, pour des tuiles, pas d'accès aux pages au hasard), et
  chaque page lue coûterait une décompression en plus du `pread`. Non essayée.
- **d. `madvise(MADV_DONTNEED)`** : une fois après la création, il ne baisse pas le pic, `VmHWM` est
  un maximum. Après chaque requête, l'essai du lot précédent donnait 15,2 MiB, avec un appel
  système par requête et du `unsafe` pour une plage de mémoire que la VFS ne mappe plus du tout.

### Coût

`unsafe` : la VFS (`projdb_exe.rs`, chaque bloc a son `// SAFETY:`), Linux seulement. Pas de nouvelle
dépendance : `rusqlite::ffi` expose `sqlite3_vfs_register`.

Temps de création de `4326 → 2154` : 533 `pread` de 4 KiB remplacent des fautes de page. À
l'instruction près, +0,9 % (callgrind : 273,0 → 275,4 millions). Au chronomètre, par processus :
médiane de 200 passes alternées 48,8 → 52,1 ms, soit environ +3 ms (+6 %), avec un bruit de ±2 ms
d'une série à l'autre. Ce coût dépasse la marge de 3 % visée : c'est le prix du `pread`, que PROJ
paie aussi. `4267 → 4326` (80 candidats) ne bouge pas dans le bruit (301 à 314 ms avant, 305 à
315 ms après). Les cas `facade_convert*` du banc ne touchent pas la base : au mieux de 3 passes,
le pire écart est +2,0 % (`facade_convert_4326_3857`, 59,5 → 60,7 ns/point), la médiane des écarts
+0,2 %.

## Les candidats partagent leurs CRS

Décision de l'opérateur : « fais partager les systèmes aux candidats ». PROJ partage ses objets par
`std::shared_ptr` (`CRSNNPtr`, `CoordinateOperationNNPtr`) ; ici chaque opération possédait ses CRS.

### Où était le tas (`4267 → 4326`, massif, pic de 16,7 MB)

Au pic, 80 opérations vivent à la fois. Classement des octets vivants par cause :

| cause | MiB | part |
|---|---:|---:|
| copies de CRS et d'opérations (`Clone`) : source, cible et interpolation de chaque candidat, de chaque étape d'une concaténation, de chaque variante « grille » | 6,7 | 42 % |
| CRS relus dans `proj.db` : `create_coordinate_reference_system` rappelé pour chaque opération et chaque étape (identifiants, domaines, membres d'ensemble de chaque datum) | 3,6 | 22 % |
| SQLite (schéma de la connexion 2,5 MiB, cache de pages 0,5) | 3,0 | 19 % |
| le reste (les opérations elles-mêmes : paramètres, identifiants, domaines) | 2,6 | 16 % |

Le même CRS (par exemple EPSG:4267 avec son ensemble de datums) existait donc en plusieurs centaines
d'exemplaires. `4326 → 2154` n'a que deux CRS et une poignée d'opérations : 3,7 MB de tas, dont
3,0 de SQLite ; rien à partager.

### Ce qui est partagé

- `source`, `target` et `interpolation` d'une `Conversion`, d'une `Transformation` et d'une
  `ConcatenatedOperation`, ainsi que la source et la cible d'un `BoundCRS`, passent de
  `Box<Crs>` à `Arc<Crs>` (`model.rs`). Cloner une opération copie quatre pointeurs au lieu de
  deux CRS entiers. Une écriture passe par `Arc::make_mut` (copie à l'écriture).
- `AuthorityFactory::create_shared_crs` : un CRS lu dans la base est construit une fois par
  recherche et par contexte (`SearchCache`, 64 entrées, clé `(autorité, code)`) et rendu en `Arc`.
  Les opérations de la base (transformations, grilles, concaténations, interpolation) l'utilisent.
  Le cache est vidé à toute écriture : `execute_sql` et une session d'insertion ne voient jamais un
  CRS d'avant. Il est vidé aussi à la fin de la dernière recherche du contexte (lot suivant :
  `docs/PORTING-creation-cache.md`, qui y range aussi les lignes des requêtes).
  `create_coordinate_reference_system` reste sans cache.
- `shared.rs` : une recherche (`create_operations_between_epochs`) enregistre ses deux `Arc<Crs>` ;
  `set_endpoints`, `with_endpoints`, `with_endpoints_of` et `set_crss_update_inverse` acceptent
  `impl Share` et retrouvent l'`Arc` quand on leur donne la `&Crs` qu'il porte (même adresse, donc
  même objet), au lieu d'en copier un. Une `&Crs` inconnue du registre est copiée, comme avant. Le
  registre est local au fil, retiré à la sortie de la recherche, et ne change aucune signature des
  121 fonctions qui passent un `&Crs`.
- Les variantes « grille alternative » de `proj_grid_substitution` gardent les `Arc` de l'opération
  d'origine au lieu de cloner source et cible. `resolve_crs` (le CRS que la recherche remplace par
  celui du registre) rend un `Arc` pris dans le cache : une recherche imbriquée retrouve le même
  objet que la recherche qui l'a lancée. Les opérations de repli (décalage géographique approximatif,
  rotation de méridien, conversion d'unité verticale) reçoivent la `&Crs` de la recherche.

Un candidat n'a plus de CRS à lui : pour `4267 → 4326`, toutes les opérations pointent vers le même
objet source et le même objet cible (`tests/shared_crs.rs`).

### Mesures

Tas (massif, binaire `--release`) et pic `VmHWM` (médiane de 5) :

| scénario | tas avant | tas après | `VmHWM` avant | `VmHWM` après | PROJ |
|---|---:|---:|---:|---:|---:|
| création `4267 → 4326` seule | 16,7 MB | **4,4** | 28,0 MiB | **14,5** | |
| `4267 → 4326` + 1 M points un par un | | | 28,0 | **14,5** | 16,3 |
| création `4326 → 2154` | 3,7 MB | 3,6 | 13,9 | 13,8 | 14,7 |

Temps, par processus (médiane de passes alternées) : création `4326 → 2154` 44,3 → 40,3 ms (200
passes ; une première série donnait 47,0 → 47,0) ; création `4267 → 4326` 307 → 113 ms (60 passes) : les copies de CRS et leurs lectures dans la base ne se font plus.
Banc (`proj-rs-bench`, 5 passes alternées, cas `facade_convert*`, sans libproj) : la meilleure passe
de chaque cas bouge de -5,4 % à +2,8 % (bruit ; le pire est `facade_convert_array_4326_3857`, dont la
médiane a trois passes à 71 à 85 ns pendant que d'autres agents chargeaient la machine), la
médiane des écarts de la meilleure passe est -1,0 %. Ces cas n'appellent pas le code modifié.

### Interface

Aucun changement d'API publique ni de type `unstable` : les champs sont `pub(crate)`, les accesseurs
(`source_crs`, `target_crs`, `interpolation_crs`) rendent toujours `Option<&Crs>`. Les bornes de
threads ne bougent pas : `Crs`, `CoordinateOperation`, `DatabaseContext` et `Context` restent
`Send + Sync`, `Transformer` et `Proj` restent `Send` (`tests/shared_crs.rs`). Les sorties (WKT,
PROJJSON, chaînes PROJ, listes `projinfo`) et les coordonnées sont celles d'avant (suites de
parité et de tests ci-dessous).

### Écarts à l'objet de PROJ

PROJ partage tout par pointeur, y compris les sous-objets d'un CRS (datum, ellipsoïde, ensemble,
système de coordonnées). Ici le CRS entier est partagé, mais un `Crs` cloné (`Crs::clone`) copie
encore ses sous-objets : un CRS reconstruit (promotion en 3D, changement d'unité) ou un `Crs` cloné à
la main reste une copie profonde. Mesuré sur `4267 → 4326`, ces copies pèsent 0,3 MiB au pic ; les
partager (`Arc<DatumEnsemble>`, `Arc<Identified>`) toucherait chaque écriture du modèle pour un gain
de cet ordre.

## Les requêtes SQL restent compilées

Décision de l'opérateur : garder les requêtes compilées. `DatabaseContext::query` appelait
`prepare` à chaque requête, et SQLite recompilait le même texte des centaines de fois par création.

### Ce qui a changé

- `DatabaseContext::query` et `towgs84_methods` passent par `prepare_cached` (rusqlite), comme
  les trois requêtes répétées de `network.rs` (`old_grid_filename`, `proj_grid_filename`,
  `direct_url`) qui lisent la même connexion par `projdb::with_db`. Les requêtes bâties à la volée
  (nom de table, clauses ajoutées selon l'autorité) ne portent que des fragments en nombre fini et
  des listes de `?` de longueur variable (pivots, autorités : une entrée par longueur, que l'ordre
  d'ancienneté évince) ; une seule inlinait une valeur : la date de publication du critère `criterionOnIntermediateCRS`
  (`createFromCRSCodesWithIntermediates`), une entrée de cache par date. Elle est maintenant un
  paramètre lié (`?`), comme les autres valeurs. Aucune requête n'est exclue du cache.
- La capacité du cache passe de 16 (défaut de rusqlite) à 64 (`projdb::STATEMENT_CACHE`) sur la
  connexion partagée, sur celle d'un contexte qui écrit et sur celles de `from_connection`.
- Chaque connexion a son cache. Une écriture (`execute_sql`, session d'insertion) prend une copie
  propre au contexte (voir plus haut) : ses requêtes se compilent sur cette copie, la connexion
  partagée garde les siennes et ne voit pas l'écriture. SQLite recompile seul une requête dont le
  schéma a changé (`SQLITE_SCHEMA`) ; `tests/statement_cache.rs` lit après une écriture de lignes,
  après un `ALTER TABLE`, entre deux contextes et autour d'une recherche.
- Le cache d'un contexte est vidé à la sortie de la recherche la plus externe
  (`create_operations_between_epochs`, `FlushStatements`) : un contexte que l'appelant garde ne
  tient aucune requête compilée au repos (mesuré, `mallinfo2` après une recherche
  `4267 → 4326` sur un contexte gardé : 4 014 384 octets avant, 4 011 760 après). La connexion
  partagée se ferme déjà avec le dernier contexte du fil, et le cache avec elle.

### Mesures

Requêtes (`callgrind`, appels de `sqlite3Prepare`) et instructions, binaire `--release` :

| scénario | compilations avant | après | instructions avant | après |
|---|---:|---:|---:|---:|
| création `4326 → 2154` | 644 | **23** | 250 M | **118 M** |
| création `4267 → 4326` | 2 699 | **46** | 904 M | **641 M** |
| 30 paires de CRS (`many`) | 46 935 | **1 003** | 18,3 G | **8,6 G** |

`4267 → 4326` compte 29 textes distincts passés à `query` (2 141 appels), plus 556 appels de
`network.rs` et 1 de `towgs84_methods` ; 21 textes distincts pour `4326 → 2154` (642 appels). Sur
30 paires, 72 textes distincts. Le chiffre de 15 604 compilations de l'opérateur est celui d'avant
le partage des CRS des candidats : ce lot part de 2 699 pour cette paire.

Temps de création, par processus (démarrage de 2,5 ms compris ; médiane de passes alternées
base / nouveau) :

| paire | passes | avant | après |
|---|---:|---:|---:|
| `4326 → 2154` | 200 | 42,0 ms | **21,7** |
| `2154 → 32631` | 200 | 45,5 | **22,8** |
| `4267 → 4326` | 100 | 121,8 | **79,2** |

Pic `VmHWM`, médiane de 5 passes (un processus par scénario), médiane de 5 séries ; PROJ 9.9.0 :

| scénario | PROJ | avant | après |
|---|---:|---:|---:|
| démarrage | 5,6 MiB | 4,6 | 4,6 |
| création `4326 → 2154` | 14,7 | 13,7 | **13,8** |
| création + 1 M points un par un | 14,8 | 13,8 | **13,9** |
| création + 1 M points en tableau | 29,8 | 29,1 | **29,1** |
| `4267 → 4326` + 1 M points | 16,3 | 14,6 | **14,7** |

Les requêtes compilées coûtent de 0,1 à 0,2 MiB de pic (massif, tas Rust et SQLite : +157 Ko pour
`4326 → 2154`, +257 Ko pour `4267 → 4326`), dans le bruit d'une série à l'autre (±0,2 MiB). Aucun
scénario ne repasse au-dessus de PROJ.

Capacité du cache (`callgrind`, `PROJ_STMT_CACHE` temporaire) :

| capacité | compilations `4326 → 2154` | `4267 → 4326` | 30 paires | instructions, 30 paires |
|---:|---:|---:|---:|---:|
| 0 (rien en cache) | 644 | 2 699 | 46 935 | 18,3 G |
| 8 | 168 | 198 | 6 019 | 10,0 G |
| 16 | 23 | 61 | 1 339 | 8,70 G |
| 32 | 23 | 46 | 1 021 | 8,60 G |
| **64 (livré)** | 23 | 46 | 1 003 | 8,59 G |

Le pic `VmHWM` ne distingue pas ces capacités (14,0 à 14,2 MiB pour la création, 14,7 à 14,9 pour
`4267 → 4326`, bruit de ±0,2 MiB). 16 suffit presque ; 64 laisse de la place à une recherche qui
utilise plus de textes distincts, sans coût mesurable : le cache est vidé à la fin de la recherche.

Banc (`proj-rs-bench`, sans libproj, 5 passes alternées, cas `facade_convert*`) : la meilleure
passe de chaque cas bouge de -4,8 % à +0,9 %, médiane des écarts -0,6 %. Ces cas n'appellent pas
la base.

## Ce qui reste, chiffres à l'appui

Aucun scénario mesuré ne reste au-dessus de PROJ. Ce qui pèse encore :

- **Le tas de `create`.** Environ 4 MiB : le schéma de `proj.db` analysé par SQLite (2,4 MiB par
  connexion), le cache de pages (0,5 MiB) et le modèle ISO 19111 d'une paire de CRS.
- **Le contrat de `convert_array` n'est plus un plancher.** La garantie « tranche intacte en cas
  d'erreur » est abandonnée (décision de l'opérateur) : le scénario « création + 1 M points en
  tableau » passe de 44,7 à 29,3 MiB (PROJ : 29,8), médiane de 5, `sum` identique
  (`7182541317022.373047`). Il ne reste que le tampon de l'appelant et le bloc de 128 Kio.
- **Les 80 candidats de `4267 → 4326`.** Réglé : voir « Les candidats partagent leurs CRS ». Il
  reste 1,2 MiB de tas Rust au pic, plus 3,0 MiB de SQLite.

## La VFS et la règle « aucun `unsafe` » : elle reste, chiffres à l'appui

`src/projdb_exe.rs` garde ses 48 `unsafe` (`tools/unsafe-debt.txt`). Aucune alternative examinée ne
garde à la fois la mémoire, la plateforme minimale (rust-version 1.83) et une API sûre.

- **Lire les octets en place** (`deserialize_bytes`, le repli actuel) n'a pas de `unsafe` dans le
  dépôt, mais touche les pages mappées du binaire. Pic `ru_maxrss` de `cs2cs` (une conversion d'un
  point, binaire `--release`, 5 passes, KiB) : `4326 → 2154` 15 368 à 15 636 avec la VFS,
  19 860 à 20 228 en place (+4,4 MiB) ; `4267 → 4326` 16 348 à 16 604 avec la VFS, 21 424 à
  22 160 en place (+5,2 MiB). Refusé : hausse de RSS.
- **Copier la base** (`deserialize_read_exact`, `Backup`, fichier ou `memfd` à part) : 10 MiB de
  tas ou de mémoire partagée par processus, ou un fichier à écrire (option b plus haut). Refusé.
- **`sqlite-plugin` 0.11** (orbitinghail, MIT / Apache-2.0, maintenu) : la seule crate trouvée dont
  l'enregistrement (`register_static`) et le trait `Vfs` sont sûrs et qui lie le SQLite de
  `libsqlite3-sys`. Elle est en édition 2024, `rust-version = 1.85` (toutes ses versions, de 0.1 à
  0.11) : `cargo +1.83.0 check` échoue (« feature `edition2024` is required »). Refusée tant que le
  plancher du dépôt est 1.83.
- **`sqlite-vfs` 0.2.0** (dernière version en 2022) : API sûre, mais l'état du VFS est rendu en
  `&mut` à chaque appel depuis un pointeur partagé (`vfs_state`), et `open` incrémente un compteur
  sans synchronisation : deux fils qui ouvrent une connexion en même temps (ce que fait le dépôt,
  une connexion par fil) font une course de données. Son `xRead` rend `SQLITE_IOERR_SHORT_READ`
  sans remplir de zéros la fin du tampon, ce que SQLite exige, et les fichiers temporaires ne
  peuvent pas retomber sur la VFS par défaut. Refusée : le défaut de sûreté serait déplacé, pas
  retiré.
- **`rsqlite-vfs` 0.2** (MIT, rust-version 1.81) : son enregistrement (`register_vfs`) et l'accès
  au fichier (`SQLiteVfsFile::handle`) sont des `unsafe fn`. Refusée : le `unsafe` resterait dans
  le dépôt.

La piste ouverte : passer à `sqlite-plugin` le jour où le plancher monte à 1.85, avec les mêmes
mesures (pic de RSS ci-dessus, `tests/shared_database.rs`, les tests de `projdb_exe`).

## Les triggers hors de l'image en lecture

Le pic de tas de la création (`4326 → 2154`, massif) est de 3,7 MiB, dont 2,6 MiB sont des structures
de schéma SQLite : `proj.db` contient 38 triggers (193 Kio de SQL, un seul fait 128 Kio). SQLite
analyse chaque trigger au premier chargement du schéma. Un trigger ne sert qu'aux insertions et aux
mises à jour (contrôles des lignes) ; une lecture ne le déclenche jamais.

- `data/proj.db` est désormais sans triggers ; `data/proj-triggers.sql` les garde, dans l'ordre
  d'origine. `tools/proj-db/split_triggers.py` fait les deux fichiers, `--restore` refait la base
  complète (le schéma rejoué est identique à celui de PROJ : 107 objets sur 107).
- `projdb::copy_into` rejoue les triggers sur chaque copie modifiable (`execute_sql`, session
  d'insertion). L'image en lecture seule n'en a aucun. Un test unitaire vérifie les deux cas.
- `tools/proj-db/update.sh` compare la base complète (rejouée) et écrit les deux fichiers.

Mesure (`perf_point … mem`, une passe, la même machine) :

| | avant | après |
|---|---:|---:|
| pic `VmHWM` après création | 12 136 kB | 9 604 kB (-2,5 MiB) |
| mémoire anonyme (tas) | 4 824 kB | 2 120 kB |
| temps de création (processus, 5 passes) | 16 ms | 11 ms |

La création d'une copie modifiable garde le même coût qu'avant, plus le rejeu des 38 triggers ; ce
coût n'est pas mesuré ici. Les tests de la bibliothèque (239), `shared_database`,
`unit_database_open`, `unit_capi_database_path`, `test_factory`, `parity`, `convert_array_golden`
et `convert_iter` passent. Le footprint de `bench/footprint.json` n'est pas refait : il attend une
mesure sur la CI.
