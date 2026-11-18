# Fiche de portage : un seul exécutable pour tous les outils

Décision de l'opérateur : PROJ partage un `libproj` et un `proj.db` entre ses outils ; proj-rs
embarque `proj.db` (10,5 Mo) et tout le code dans chaque outil, soit 144 Mo pour les huit. Le
binaire `proj-rs` (`src/bin/proj-rs.rs`) les réunit : un programme, un `proj.db`.

## Le choix de l'outil

| Lancement | Outil | Nom vu par l'outil (`argv[0]`) |
|---|---|---|
| `cs2cs ...`, lien ou copie de `proj-rs` portant ce nom | `cs2cs` | celui du processus, tel quel |
| `proj-rs cs2cs ...` | `cs2cs` | `cs2cs` |
| `proj-rs`, nom inconnu | aucun : liste des outils sur stderr, code 1 | |
| `proj-rs --list` | un nom d'outil par ligne (lu par les scripts d'installation) | |
| `proj-rs --help`, `proj-rs --version` | usage de `proj-rs`, `proj-rs: Rel. 9.9.0, ...` | |

Le nom est le dernier élément de `argv[0]`, sans `.exe`. Sous Windows il est mis en minuscules
d'abord (`CS2CS.EXE` lance `cs2cs`). Un nom qui n'est pas celui d'un outil (`proj-rs`,
`proj-rs-0.1`, `foo`) laisse le premier argument choisir l'outil.

Les outils lisent `argv[0]` pour leur usage et leurs messages (`usage: cs2cs ...`) et pour leur
sens : `invproj` et `invgeod` tournent à l'envers parce que leur nom commence par `inv`, comme
`basename(argv[0])` dans PROJ. Pour que `proj-rs invproj` se comporte de même, chaque outil a un
point d'entrée qui prend ses arguments (`run_with(argv)` pour `cct`, `cs2cs`, `gie`, `projinfo`,
`projsync`; `run(argv)` pour `proj_cli` et `geod_cli`) au lieu de lire `std::env::args()`. Les
binaires séparés gardent leur `fn main` : un `[[bin]]` de chaque outil reste dans `Cargo.toml`,
et c'est lui que les tests lancent par `CARGO_BIN_EXE_<nom>`. `src/bin/proj-rs.rs` inclut les
mêmes fichiers (`#[path]`), donc le code des outils n'existe qu'une fois dans les sources. Le
binaire `proj-rs` a `test = false` : les tests unitaires des outils tournent dans leurs binaires,
pas deux fois.

## Installation

`tools/dist-cli.sh [--release] [PREFIX]` (Linux, macOS) installe `PREFIX/bin/proj-rs` et un lien
symbolique relatif par outil. `tools/dist-cli.ps1 [-Release] [-Prefix] [-Launchers]` (Windows) :

| Système | Nom d'un outil | Pourquoi |
|---|---|---|
| Linux, macOS | lien symbolique `cs2cs -> proj-rs` | `argv[0]` est le nom du lien |
| Windows, NTFS, même volume | lien physique `cs2cs.exe` | un vrai `.exe`, aucun octet de plus ; `argv[0]` est le nom tapé |
| Windows, sinon (ou `-Launchers`) | `cs2cs.cmd` : `"%~dp0proj-rs.exe" cs2cs %*` | repli : une copie par nom coûterait 18 Mo chacune |

Limites du `.cmd` : ce n'est pas un exécutable. Un programme qui lance `cs2cs` sans shell
(`CreateProcess`, `subprocess.run(["cs2cs"])`) ne le trouve pas ; Ctrl+C demande « Terminer le
programme de commandes ? » ; cmd développe `%VAR%`, `^` et `!` dans les arguments avant le
lancement ; le code de retour passe (`exit /b %ERRORLEVEL%`), stdin est hérité. Un lien physique
copié par l'Explorateur ou `Copy-Item` devient une copie complète : relancer le script à la
destination. Non testé ici : aucune machine Windows, seulement `cargo check` de la cible
`x86_64-pc-windows-gnu`.

## Taille

Linux x86-64, `release`, `strip --strip-unneeded`, même toolchain (stable 1.100) partout. Les huit
binaires séparés : 144 445 920 octets (137,8 Mio). `proj-rs` seul, profil `release` : 19 257 752.
Où va le reste : `.rodata` 10,89 Mo (dont `proj.db` 10,55 Mo), `.text` 7,16 Mo (`proj_rs` 4,5 Mo
en 3 200 fonctions, la plus grosse 37 Ko ; SQLite 1,3 Mo ; `std` 0,7 Mo ; `tiff` 0,1 Mo),
`.eh_frame` 0,50 Mo, `.rela.dyn` 0,27 Mo, `.gcc_except_table` 0,18 Mo. `cargo bloat` ne montre
aucune fonction ni aucune monomorphisation dupliquée qu'un correctif simple retirerait.

| Étape (cumulée), `proj-rs` | Octets | Écart |
|---|---:|---:|
| `release` (LTO thin, unwind), SQLite par défaut | 19 257 752 | |
| + `panic = "abort"` | 18 686 232 | -571 520 |
| + `lto = "fat"` | 18 579 096 | -107 136 |
| + SQLite allégée (`.cargo/config.toml`) | 17 978 968 | -600 128 |
| `strip = true` : taille de sortie, sans repasser par `strip` | 17 979 008 (contre 20 221 392) | -2 242 384 |

Total : 144,4 Mo en huit binaires, 18,6 Mo en un (-87,2 %) sans `panic = "abort"` (voir plus bas) ; 18,0 Mo avec.

**Profil `release-cli`** (`Cargo.toml`) : `release` plus `lto = "fat"` et `strip = true`. Les
scripts `tools/dist-cli.*` l'utilisent avec `--release`. LTO fat n'a pas coûté de temps de build
mesurable (2 min 32 contre 2 min 36, build complet à froid).

**`panic = "abort"` n'est pas retenu** (décision du coordinateur, opérateur absent), malgré ses
-571 520 octets. `src/projections/gridshift/geotiff/mod.rs` (`contained`) rattrape par
`catch_unwind` un panic de la crate `tiff` 0.10.3 (tag `SampleFormat` sans valeur) et le rend comme
un fichier de grille invalide : sous `abort`, un `.tif` malformé tuerait l'outil (code 134) au lieu
de donner l'erreur, contre la règle « fail closed ». À reconsidérer quand `tiff` 0.11, qui vérifie
ce tag, sera adopté (il exige Rust 1.85). La capi garde de toute façon `unwind` : son garde
`catch_unwind` (`capi/src/util.rs`) en dépend. Taille mesurée sans `abort` : 18 551 520 octets.

**SQLite allégée.** `.cargo/config.toml` pose `LIBSQLITE3_FLAGS` (lu par `libsqlite3-sys`
0.35, qui l'ajoute après ses défauts) : sans FTS3/FTS5, R-tree, dbstat, soundex, métadonnées de
colonnes, extensions chargeables ; avec `OMIT_DEPRECATED`, `OMIT_SHARED_CACHE`,
`OMIT_PROGRESS_CALLBACK`. `proj.db` n'a ni table virtuelle ni JSON ; ses 38 triggers, ses vues et
`sqlite_stat1` restent servis. Tous les builds du dépôt la prennent (suite de tests comprise,
et la bibliothèque C) ; une crate qui dépend de proj-rs ne lit pas ce fichier et garde la SQLite
par défaut. Un second jeu a été mesuré et refusé : `OMIT_AUTHORIZATION`, `TRACE`, `INCRBLOB`,
`COMPLETE`, `TCL_VARIABLE`, `GET_TABLE`, `JSON`, `UTF16` ne gagnent que 75 520 octets de plus
(17 903 448) et fermeraient les bases auxiliaires en UTF-16.

**Rejeté sans mesure** : `-C relocation-model=static` (retirerait `.rela.dyn`, 0,27 Mo, mais
supprime l'ASLR) ; `opt-level` (la vitesse est l'argument du projet).

## Vitesse

La VM est partagée : `proj-rs-bench` varie de plus de 20 % d'un cas à l'autre entre deux
passes identiques, même au minimum de 12 passes alternées (`taskset -c 3 nice -n -10`), donc le
seuil de 3 % ne se lit pas sur un cas. Médiane des 214 cas contre le binaire de `main` :
`release` avec la SQLite allégée +0,1 %, `release-cli` +0,1 % ; sur les 10 cas `facade_convert*`,
médiane +4,8 % et -1,1 % avec un pire cas à +22 % et +20 % : non concluant à ce niveau de
bruit. La mesure de bout en bout des programmes est moins bruitée (10 et 40 passes alternées, temps
CPU, `proj-rs cs2cs` de `main` contre `release-cli`) : 1 000 000 points EPSG:4326 vers 2154,
minimum 0,706 s contre 0,631 s ; un point EPSG:4267 vers 4326 (candidats, base SQL), minimum
0,049 s contre 0,047 s. Aucune régression visible, pas de preuve à 3 % non plus.

## Dossiers de données

Les binaires ne lisent plus `data/proj.db` par le chemin de la machine de build
(`env!("CARGO_MANIFEST_DIR")` dans `cct` pour `operation_from_name` et dans `cs2cs` pour
`Catalog::open`) : ils passent par `proj_rs::embedded_proj_db`, la base embarquée. Le
`--system-directory` de `projsync` suit PROJ (`projsync.cpp` 147-153) par
`proj_rs::system_data_directory` : `../share/proj` du module, sinon le dossier compilé
(`PROJ_RS_DATA_DIR`), sinon le dossier utilisateur (`projsync.cpp` 238-246). La recherche des
grilles suit `src/datadir.rs` (`docs/PORTING-data-dirs.md`).
`std::env::current_exe` donne, avec le multi-appel, le chemin de `proj-rs` : un lien
symbolique est résolu vers sa cible (Linux ; sous macOS non vérifié ici), un lien physique ou
un lanceur `.cmd` de Windows reste dans `bin/`. Les scripts `dist-cli.*` posent tous les noms à
côté de `proj-rs`, donc `<prefix>/bin/../share/proj` reste `<prefix>/share/proj`.

## Vérifié à l'identique

`tests/cli_multicall.rs` lance 22 commandes (usage sans argument, `--version`, option inconnue,
CRS inconnu, conversions avec stdin, `proj`/`invproj`, `geod`/`invgeod`, `cct`, `projinfo`, `gie`) de
trois façons, binaire séparé, `proj-rs <outil>` et `proj-rs` sous le nom de l'outil, et exige le
même stdout, le même stderr et le même code de retour. Le reste de la suite lance toujours les
binaires séparés.
