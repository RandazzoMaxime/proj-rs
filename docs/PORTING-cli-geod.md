# Port — CLI `geod`, `invgeod`, `invproj` et yaml nad27/nad83

Sources : PROJ 9.9.0 `src/apps/geod.cpp`, `geod_set.cpp`, `geod_interface.cpp`,
`proj.cpp`, `utils.cpp` ; `src/dmstor.cpp`, `src/rtodms.cpp`, `src/adjlon.cpp` ;
`src/conversions/cart.cpp` ; `src/test/cli/` (tests yaml et
`test_cs2cs_locale.sh`). Audit : [AUDIT-cli-geod.md](AUDIT-cli-geod.md).

## Carte

| Fichier | Rôle |
|---|---|
| `src/geod.rs` | API publique `Geod`, `Line`, `Inverse` : Karney en degrés, mètres (`geod_init`, `geod_inverse`, `geod_lineinit` sans capacité, `geod_position`). |
| `src/geodesic.rs` | Ajouts `Geodesic::new`, `inverse_full`, `GeodesicLine::plain`, `position_azimuth`, autour du C déjà compilé. |
| `src/bin/geod.rs`, `src/bin/invgeod.rs` | Points d'entrée (`geod_cli::run`). |
| `src/bin/invproj.rs` | Point d'entrée (`proj_cli::run`). |
| `src/bin/proj.rs` | Réécrit en point d'entrée sur `proj_cli` (il gère `invproj` par le nom du programme, comme PROJ). |
| `src/bin/support/dms.rs` | `dmstor`, `c_strtod`, `rtodms` / `Dms` (`set_rtodms`), `format_number` (`limited_fprintf_for_number`), `Direction`, `program_name`, `direction_of`. |
| `src/bin/support/geod_cli.rs` | `geod.cpp` : options, `geod_set`, `geod_pre` / `geod_for` / `geod_inv`, `do_arc`, `do_geod`, `process`, lecture `fgets` de 200 octets. |
| `src/bin/support/proj_cli.rs` | `proj.cpp` : options, lecture (DMS en sens direct, `strtod` en sens inverse), `-E`, `-S`, `-f`, `-t`, `-w`/`-W`. |
| `src/crs.rs` | `+init=fichier:section` : lecture des sections `<n> ... <>` dans le chemin `PROJ_DATA` (`init_file_section`). |
| `src/engine.rs` | `Operation::create_projection` (voir écart 1) ; `cartesian_to_geodetic` calcule comme `cart.cpp` (voir écart 3). |
| `tests/cli_yaml.rs` | `FILES` : nad27, nad83, invproj ; `invproj` dans `run_case` ; `cs2cs_locale_script`. |
| `tests/cli_geod.rs` | 30 cas `geod` / `invgeod` dont les sorties viennent du binaire de référence. |
| `tests/cli_yaml/` | `test_invproj.yaml`, `test_proj_nad27.yaml`, `test_proj_nad83.yaml`, `test_cs2cs_locale_out.dist`. |
| `data/tests/nad27`, `data/tests/nad83` | Fichiers `+init` copiés de `src/data/` de PROJ 9.9.0 (licence MIT de PROJ). |
| `Cargo.toml` | Trois `[[bin]]` : `invproj`, `geod`, `invgeod`. Aucune dépendance ajoutée. |

## Écarts volontaires vers PROJ

1. **`proj` et `invproj` n'appliquent aucun décalage de datum.** PROJ lance
   `pj_fwd` (la seule projection). `Operation::create` ajoute en revanche le
   décalage de grille de `+datum=NAD27` (`+nadgrids`). Le nouveau constructeur
   `Operation::create_projection` garde la même voie de construction et retire
   seulement `towgs84`, `nadgrids` et `geoidgrids` après l'expansion de
   `+datum` et de `+init` ; l'ellipsoïde du datum est gardé. Sans cela,
   `test_proj_nad27.yaml` donne ~350 pieds d'écart, dès que la grille `conus`
   est sur le chemin.
2. **`+init=fichier:section` lit le fichier sur le chemin de données.** Le
   cas `nad27:3901` garde sa définition intégrée, comme avant, quand le
   fichier n'est pas là.
3. **Hauteur géocentrique au pôle.** `cart.cpp` multiplie chaque norme par son
   inverse (`c = p * inv_norm`, etc.). La crate divisait ; le signe d'une
   hauteur nulle changeait (`-0.000` au lieu de `0.000`). La crate suit
   maintenant l'ordre de `cart.cpp`.
4. **Pas d'ellipsoïde par défaut dans `geod`.** Sans argument d'ellipsoïde,
   le binaire de référence échoue (`ellipsoid setup failure`) : c'est le
   comportement porté. L'hypothèse GRS80 de départ était fausse.
5. **Erreurs au format de l'exécutable.** `emess` affiche la version de PROJ
   et `<geod>`. Le port affiche `<geod>: message`. Le texte est sur stderr,
   que les tests ne comparent pas.

## Non porté

- `geod -l`, `-le`, `-lu` et `+sunits` : les listes d'ellipsoïdes et d'unités
  de PROJ n'ont pas d'équivalent public dans la crate (détail dans l'audit).
- Options de `proj.cpp` autres que `-E -I -S -f -t -w -W -h` (binaire, `-r`,
  `-s`, `-d`, `-e`, `-i`, `-l`, `-m`, `-o`, `-V`).
- Facteurs : omega peut sortir à `0` là où PROJ sort `8.54059e-07` (arrondi
  machine, comme dans `test_proj.yaml`).

## Commandes

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --test cli_yaml        # nad27, nad83, invproj, locale, + les autres yaml
cargo test --test cli_geod        # geod / invgeod contre la sortie du binaire de référence
cargo +1.100.0 clippy --all-targets --all-features -- -D warnings
```

Comparaison à la main avec le binaire de référence (même `PROJ_DATA`) :

```bash
R=target/proj-reference/build
PROJ_DATA=$R/data LD_LIBRARY_PATH=$R/lib $R/bin/geod +ellps=GRS80 -a
```
