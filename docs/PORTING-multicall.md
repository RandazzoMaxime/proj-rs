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

Linux x86-64, `release`, `strip --strip-unneeded`, même toolchain des deux côtés :

| | Huit binaires séparés | `proj-rs` + liens |
|---|---:|---:|
| Octets | 144 445 920 (137,8 Mio) | 19 257 752 (18,4 Mio) |

## Vérifié à l'identique

`tests/cli_multicall.rs` lance 22 commandes (usage sans argument, `--version`, option inconnue,
CRS inconnu, conversions avec stdin, `proj`/`invproj`, `geod`/`invgeod`, `cct`, `projinfo`, `gie`) de
trois façons, binaire séparé, `proj-rs <outil>` et `proj-rs` sous le nom de l'outil, et exige le
même stdout, le même stderr et le même code de retour. Le reste de la suite lance toujours les
binaires séparés.
