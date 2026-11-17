# Mémoire : où elle va, ce qui a changé, ce qui reste

Ce lot ne porte pas de code PROJ. Il mesure le pic de mémoire résidente (`VmHWM`) de proj-rs et
retire les copies que le code faisait sans nécessité. Le contrôle des résultats est inchangé : aucun
chiffre, aucune erreur, aucun ordre de candidats ne bouge.

## Mesure

`bench/footprint.sh` et les programmes de `bench/footprint/` (le même programme en Rust et en C
contre libproj). Linux x86-64, médiane de 5 passes, pic `VmHWM`, un processus par scénario.

| scénario | PROJ 9.9.0 | avant | après |
|---|---:|---:|---:|
| démarrage | 5,6 MiB | 4,6 | 4,8 |
| création `4326 → 2154` | 14,7 | 34,3 | 18,6 |
| création + 1 M points un par un | 14,8 | 34,5 | 18,6 |
| création + 1 M points en un tableau | 29,8 | 95,4 | 49,3 |
| `4267 → 4326` (80 candidats) + 1 M points | 16,3 | 51,3 | 33,9 |

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

## Ce qui reste, chiffres à l'appui

Aucun scénario ne passe sous PROJ. Ce qui l'empêche :

- **Pages de fichier de la base embarquée.** `create` : 18,3 MiB au total, dont 5 MiB de tas et
  13 MiB de pages de fichier (`RssFile`) : 4 MiB sont celles d'un binaire vide, le reste est du code
  et des pages de `proj.db` lues par les requêtes. PROJ lit sa base avec `read`, et les pages de
  cache du noyau ne comptent pas dans son `VmHWM`. Un `madvise(MADV_DONTNEED)` de la base après
  chaque requête (essai, non livré) ramène `create` à 15,2 MiB, au-dessus des 14,7 de PROJ, au prix
  d'un appel système par requête et de code `unsafe` propre à Linux. Lire la base par `pread` de
  l'exécutable (VFS SQLite maison) irait plus bas ; ce n'est pas fait.
- **Le contrat de `convert_array`.** « En cas d'erreur la tranche est laissée intacte » est
  testé (`tests/convert_array_batch.rs`, `tests/native_api.rs`). Il impose de garder les résultats
  (ou les originaux) jusqu'au dernier point : une tranche de 1 M de points coûte 15 MiB de plus que
  le tampon de l'appelant. PROJ convertit sur place et laisse la tranche en partie convertie. Plancher
  de ce contrat : démarrage + création + 2 fois le tampon, soit 18,6 + 30,6 = 49 MiB.
  Une variante sur place (tranche en partie convertie en cas d'erreur) tomberait à 34 MiB.
- **Les 80 candidats de `4267 → 4326`.** Chaque candidat porte ses CRS source et cible (`Box<Crs>`),
  avec les identifiants, les domaines et les membres d'ensemble de chaque datum : 12 MiB de tas au
  pic. Les partager (`Arc`) touche tout le modèle ISO 19111 ; ce n'est pas fait.
