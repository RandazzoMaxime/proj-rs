# Port — CLI `cs2cs`, `proj`, `geod` et `projinfo` : yaml restants et listes

Sources : PROJ 9.9.0 `src/apps/proj.cpp`, `geod.cpp`, `geod_set.cpp`, `cs2cs.cpp` ;
`src/init.cpp`, `src/trans.cpp`, `src/grids.cpp`, `src/ellps.cpp`, `src/list.cpp`,
`src/projections/*.cpp` (`PROJ_HEAD`) ; `src/iso19111/c_api.cpp` et `factory.cpp`
(`getUnitList`) ; `src/test/cli/` (yaml et `test_projinfo.sh`). Audit :
[AUDIT-cli-rest.md](AUDIT-cli-rest.md).

## Carte

| Fichier | Rôle |
|---|---|
| `src/crs.rs` | `section_words` : une section `<n> ... <>` d'un fichier `+init=` est lue en entier ; `ignf_definition` : `+init=IGNF:CODE` depuis proj.db. |
| `src/engine.rs` | `split_quoted` : un mot entre guillemets reste un mot (`+nadgrids="dir with space/myconus"`). |
| `src/projections/legacy.rs` | Le nom de grille d'un `+datum=` avec espaces est cité dans la chaîne reconstruite. |
| `src/projections/gridshift/ntv1.rs` | Lecteur NTv1 (`.dat`), port de `NTv1Grid::open` et `valueAt`. |
| `src/projdb.rs` | `PROJ_AUX_DB` : bases auxiliaires attachées et lignes copiées dans les tables. |
| `src/listing.rs` | Données publiques : `PROJECTIONS` (192), `ELLIPSOIDS` (46), `linear_units()` (proj.db). Générées depuis les sources de PROJ ; la table est la donnée, pas le générateur. |
| `src/bin/cs2cs.rs` | `init_side_as_code` (IGNF seulement), `init_authority`, `geoid_without_height`, `raw_words`, prise en compte des guillemets d'argument. |
| `src/bin/support/proj_cli.rs` | `-d -e -m -r -s -i -o -b -l`, contrôles `fwd_prepare` de `pj_fwd`. |
| `src/bin/support/geod_cli.rs` | `-l`, `-le`, `-lu`, `+units=`. |
| `src/bin/support/lists.rs` | Affichage des listes d'ellipsoïdes et d'unités, comme `proj.cpp` et `geod.cpp`. |
| `tests/cli_yaml.rs` | `FILES` : +4 yaml (datumfile, ntv2, ignf, flaky) ; `projinfo_aux_script`. |
| `tests/cli_yaml/` | `test_cs2cs_*.yaml`, `test_projinfo_out.dist`. |
| `data/tests/dir with space/myconus` | Copie de `conus`, comme la règle CMake de PROJ (`src/data/CMakeLists.txt`). |

## Corrections de fond

1. **`+init=FILE:SECTION` lit toute la section.** `dictionary_section` ne prenait que la
   ligne `<n>` : `+init=nad83:5002` devenait `proj=tmerc datum=NAD83`, et tous les
   défauts (`lon_0`, `lat_0`, `k`, `x_0`) s'appliquaient. Corrige les 225 cas nad83.
2. **`+init=IGNF:CODE`** est lu dans proj.db comme `+init=epsg:` (`pj_get_init_items`).
3. **`cs2cs` et les CRS de base.** Un côté `+init=IGNF:CODE` passe à la sélection ISO sous
   la forme `IGNF:CODE`. Les `+init=epsg:` restent sur le chemin des chaînes PROJ : passer
   aussi les `epsg:` par la sélection ISO a changé l'ordre des axes de
   `test_cs2cs_various.yaml#59`, `#67`, `#68` (régression mesurée, annulée).
4. **`+geoidgrids` en destination** : la destination est un CRS composé (`instantiate_crs`),
   donc des nombres bruts, pas du DMS. Sans `--3d`, une source sans hauteur ne reçoit pas le
   géoïde (la source 2D n'a pas de hauteur à corriger) ; une source 3D (`+init=epsg:4979`) ou
   `--3d` le garde.
5. **Guillemets dans les chaînes PROJ.** `pj_mkparam_ws` garde les espaces d'une valeur
   entre guillemets. L'analyseur coupait sur les espaces ; il découpe maintenant hors des
   guillemets. `cs2cs` cite un argument contenant un espace avant de rejoindre la
   définition, et ne passe pas une chaîne citée par la sélection ISO (son réimport coupe
   les espaces).
6. **NTv1.** Entête en big-endian, degrés, enregistrements d'est en ouest ; même convention
   de longitude que NTv2 (`compensateNTConvention`).

## Écarts volontaires vers PROJ

1. **`+sunits=` n'existe pas dans PROJ** : `geod_set` lit `pj_param(..., "sunits")`, et
   `pj_param` prend la première lettre comme type (`s`) : la clé lue est `units`. Ce port lit
   `+units=`, comme PROJ. L'énoncé du lot disait `+sunits=` ; la sortie de référence avec
   `+sunits=` ignore l'unité.
2. **`PROJ_AUX_DB` par copie de lignes.** PROJ attache la base et ses vues interrogent les
   deux. Ici les lignes de la base auxiliaire sont copiées dans les tables principales
   (une ligne auxiliaire remplace la ligne de même clé). Une vue temporaire par table ne
   suffit pas : les vues de la base principale lisent les tables principales. `usage` est
   copiée en dernier (son déclencheur `usage_insert_trigger` vérifie les objets). `metadata`
   n'est pas copiée : chaque base contrôle son niveau de schéma.
3. **Contrôles `fwd_prepare` dans la CLI seulement.** `pj_fwd` refuse une latitude au-delà du
   pôle et une longitude au-delà de 10 rad. Ici seul `proj` le fait (`Job::project`) ;
   `Operation::forward` ne le fait pas. `cs2cs` et `cct` ont donc le même écart.
4. **Texte d'erreur d'`emess`.** Le port écrit `<prog>: message` ; PROJ écrit en plus l'en-tête
   `Rel. 9.9.0 ...` et la fin `program abnormally terminated`. Seul le code de sortie est
   comparé.

## Écarts ouverts

| Écart | Cause exacte |
|---|---|
| `test_cs2cs_ignf.yaml#9`, `#17` | Voir AUDIT : candidat `towgs84` du CRS lié absent de la liste des candidats. |
| `test_cs2cs_datumfile.yaml#3` (`--bbox`) | Voir AUDIT : le chemin `facade` est sensible à l'aire mais change `--area`. |
| `proj -S` omega à la latitude -80 (`100 -80`, `+ellps=GRS80` Mercator) | PROJ dérive la projection unitaire (`fwd` sans `fwd_finalize`, `deriv.cpp`) ; ici la dérivée passe par les mètres de `forward`. Les points conformes donnent 0, comme PROJ ; à -80 l'écart est 5e-7 rad. |
| `proj -b/-i/-o` : derniers bits | L'ordre des opérations de `tmerc`/`utm` inverse et de Mercator diffère de PROJ : écart d'un à deux ulps, non visible en texte. |
| `proj -v`, `-V` | Non porté : le dump de moniteur (`pj_pr_list`, `vprocess`) est une sortie interne de PROJ. |

## Commandes

```bash
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
R=/chemin/vers/target/proj-reference
PROJ_RS_LIBPROJ_DIR=$R/build/lib cargo test --test cli_yaml     # yaml et projinfo_aux_script
```

Comparaison à la main avec le binaire de référence, sur les mêmes arguments :

```bash
PROJ_DATA=$R/build/data:$PWD/data:$PWD/data/tests LD_LIBRARY_PATH=$R/build/lib \
  $R/build/bin/proj -l
```
