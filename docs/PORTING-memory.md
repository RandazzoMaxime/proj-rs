# Mémoire : où elle va, ce qui a changé, ce qui reste

Ce lot ne porte pas de code PROJ. Il mesure le pic de mémoire résidente (`VmHWM`) de proj-rs et
retire les copies que le code faisait sans nécessité. Le contrôle des résultats est inchangé : aucun
chiffre, aucune erreur, aucun ordre de candidats ne bouge.

## Mesure

`bench/footprint.sh` et les programmes de `bench/footprint/` (le même programme en Rust et en C
contre libproj). Linux x86-64, médiane de 5 passes, pic `VmHWM`, un processus par scénario.

| scénario | PROJ 9.9.0 | avant | après #228 | après ce lot |
|---|---:|---:|---:|---:|
| démarrage | 5,6 MiB | 4,6 | 4,8 | 4,7 |
| création `4326 → 2154` | 14,7 | 34,3 | 18,6 | **13,9** |
| création + 1 M points un par un | 14,8 | 34,5 | 18,6 | **13,9** |
| création + 1 M points en un tableau | 29,8 | 95,4 | 49,3 | 44,2 |
| `4267 → 4326` (80 candidats) + 1 M points | 16,3 | 51,3 | 33,9 | 28,0 |

Les colonnes « avant » et « après #228 » sont celles du lot précédent. La dernière a été mesurée
avec le même programme, sur la même machine, juste après une remesure de l'état de départ
(4,7 / 18,4 / 18,6 / 48,9 / 33,6). Sous Linux, la création d'une paire de CRS passe sous PROJ. Les
deux scénarios à un million de points en tableau et à 80 candidats restent au-dessus : leurs
causes sont en fin de page.

## Où allaient les octets

massif (tas), `smaps` (pages de fichier) et dhat sur chaque scénario.

1. **Une copie de `proj.db` par `DatabaseContext::create()`.** La base embarquée fait 10 MiB. Chaque
   contexte la recopiait dans une base mémoire modifiable (`Backup`), qui lisait aussi toutes les
   pages de la base dans le binaire : 10 MiB de tas plus 10 MiB de pages de fichier. Un contexte
   n'écrit presque jamais (une session d'insertion, `execute_sql`). La création d'une paire de CRS en
   ouvre plusieurs, imbriqués.
2. **Un tableau converti = 3 copies.** `convert_array` construisait un `Vec<Xyzt>` de toute la
   tranche (32 octets par point, soit 31 MiB pour 1 M de points), puis un `Vec<C>` des résultats
   (15 MiB), pour garantir qu'une erreur laisse la tranche intacte.
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
- `convert_array` travaille par blocs de 4096 points (un seul bloc de `Xyzt` vivant) et ne garde
  que les résultats, dans leur type : 31 MiB de moins pour 1 M de points. L'ordre des erreurs est
  celui d'avant : conversion de l'entrée, puis erreur de transformation du premier point en échec,
  puis rétrécissement vers le type de la tranche.
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

## Ce qui reste, chiffres à l'appui

Deux scénarios restent au-dessus de PROJ. Ce qui l'empêche :

- **Le tas de `create`.** Environ 5 MiB : le schéma de `proj.db` analysé par SQLite (2,4 MiB par
  connexion), le cache de pages (0,5 MiB) et le modèle ISO 19111 d'une paire de CRS.
- **Le contrat de `convert_array`.** « En cas d'erreur la tranche est laissée intacte » est
  testé (`tests/convert_array_batch.rs`, `tests/native_api.rs`). Il impose de garder les résultats
  (ou les originaux) jusqu'au dernier point : une tranche de 1 M de points coûte 15 MiB de plus que
  le tampon de l'appelant. PROJ convertit sur place et laisse la tranche en partie convertie. Plancher
  de ce contrat : démarrage + création + 2 fois le tampon, soit 13,9 + 30,6 = 44 MiB.
  Une variante sur place (tranche en partie convertie en cas d'erreur) tomberait à 29 MiB, au niveau
  de PROJ (29,8).
- **Les 80 candidats de `4267 → 4326`.** Chaque candidat porte ses CRS source et cible (`Box<Crs>`),
  avec les identifiants, les domaines et les membres d'ensemble de chaque datum : 12 MiB de tas au
  pic. Les partager (`Arc`) touche tout le modèle ISO 19111 ; ce n'est pas fait.
