# Audit du lot cli-geod

Sources lues (lecture seule) : `target/proj-reference/src/src/apps/` (`geod.cpp`,
`geod_set.cpp`, `geod_interface.cpp`, `proj.cpp`, `utils.cpp`, `emess.cpp`),
`src/src/dmstor.cpp`, `src/src/rtodms.cpp`, `src/src/adjlon.cpp`,
`src/src/conversions/cart.cpp`, et `src/test/cli/` (`test_invproj.yaml`,
`test_proj_nad27.yaml`, `test_proj_nad83.yaml`, `test_cs2cs_locale.sh`,
`test_cs2cs_locale_out.dist`, `CMakeLists.txt`).

Les cas du yaml sont exécutés par `tests/cli_yaml.rs`, qui lit
`docs/UNPORTED-CLI.md` : après ce lot, ce fichier ne contient plus aucune ligne.

## Fichiers de test de PROJ 9.9.0 : `test/cli/`

| Source PROJ | Cas | Test Rust | Résultat |
|---|---|---|---|
| `test_invproj.yaml` | 1 | `tests/cli_yaml.rs` (`FILES`, exe `invproj`) | passe |
| `test_proj_nad27.yaml` | 265 | `tests/cli_yaml.rs` (`FILES`) | passe |
| `test_proj_nad83.yaml` | 225 | `tests/cli_yaml.rs` (`FILES`) | passe |
| `test_cs2cs_locale.sh` | 6 exécutions, comparées à `test_cs2cs_locale_out.dist` | `tests/cli_yaml.rs::cs2cs_locale_script` | passe |

Précisions sur `test_cs2cs_locale.sh` :

- Le script ne prend la variante « virgule décimale » que si la locale
  `fr_FR.utf8` existe. Ce banc n'a pas cette locale : seul le cas par défaut
  est exécuté.
- La comparaison suit `diff -u -b` : les espaces (quantité) sont ignorés. Les
  règles `####` ont une largeur qui varie d'une section à l'autre dans le
  script ; une règle est comparée comme une règle, quelle que soit sa largeur.
- Ce test a révélé un écart réel, corrigé : la hauteur d'un point au pôle
  sortait `-0.000` au lieu de `0.000` (voir `PORTING-cli-geod.md`).

## `geod` et `invgeod` : cas de PROJ

`grep -rn 'geod' target/proj-reference/src/src/test/` ne trouve aucun cas
`geod` ou `invgeod` dans les yaml, les scripts shell ni `testvarious` de
PROJ 9.9.0. Le fichier `testvarious` demandé n'existe pas dans ce source. Les
tests de `geod` ajoutés (`tests/cli_geod.rs`, 30 cas) ne sont donc pas des
cas de PROJ : leurs sorties attendues viennent du binaire de référence
`target/proj-reference/build/bin/geod`, lancé sur les mêmes arguments.

| Comportement de `geod` | Test |
|---|---|
| Sens direct, sens inverse (`invgeod`, `-I`) | `forward_*`, `inverse_*`, `inverse_flag_I_on_geod` |
| `-a`, `-f`, `-F`, `-p`, `-t`, `-w`, `-W` | `forward_full`, `forward_decimal`, `forward_positive_azimuth_wgs84`, `forward_seconds_*`, `forward_tag_line_copied`, `forward_distance_format_and_tail` |
| Reste de ligne recopié après les valeurs | `forward_distance_format_and_tail`, `forward_missing_distance_is_zero` |
| Entrée en degrés-minutes-secondes | `forward_dms_input` |
| Ellipsoïde : `+ellps`, `+a`/`+f`, `+a`/`+rf`, `+R` | `forward_custom_axes`, `inverse_rf_axes`, `forward_sphere`, `forward_semi_major_only` |
| Modes arc et ligne (`+lat_1`, `+A`, `+n_A`, `+del_A`, `+lat_2`, `+n_S`, `+del_S`) | `arc_of_azimuths`, `line_*` |
| Erreurs : pas d'argument `+`, pas d'ellipsoïde, option inconnue, ellipsoïde inconnu, fichier avec mode ligne, diviseur absent, `+A` sans `+S` | `error_*` |

Ces erreurs ont été trouvées en comparant au binaire de référence. Le binaire
ne sait pas se passer d'ellipsoïde (`ellipsoid setup failure`) : l'ellipsoïde
GRS80 par défaut, présumé au départ, n'existe pas. Sans argument `+`, il
signale `no arguments in initialization list`.

## Options de `proj` et `invproj`

| Option ou comportement | Statut |
|---|---|
| `invproj` = `proj` en sens inverse (nom du programme `inv...`, `-I`) | porté, `test_invproj.yaml` (tests/cli_yaml.rs) |
| Entrée directe en DMS, reste de ligne recopié, `-E`, `-f`, `-t`, `-w`, `-W` | porté, couvert par les yaml nad27/nad83 et les comparaisons au binaire |
| `-S` : facteurs calculés aux valeurs lues, en sens inverse aussi | porté ; en sens inverse, les valeurs x,y lues comme radians donnent `* * * * * *`, comme PROJ |
| `-h` / usage | porté (texte différent, hors comparaison : stderr) |

## Écarts restants

| Écart | Cause exacte |
|---|---|
| `geod -l`, `geod -le`, `geod -lu` (listes d'ellipsoïdes et d'unités) | Pas de liste publique des ellipsoïdes (`proj_list_ellps`) ni des unités (`proj_get_units_from_database`) dans la crate. Erreur `invalid option` au lieu d'une liste. |
| `geod +sunits=…` (unité de la distance autre que le mètre) | La table d'unités du moteur (`engine.rs`, `linear_unit`) ne contient que les noms `+units` ; PROJ lit `proj.db`. La crate n'expose pas cette table. Le programme ne prend donc que le mètre. |
| `proj -S` : angle de distorsion (omega) à `0` au lieu de `8.54059e-07` sur la même machine (constaté avec `+proj=utm +zone=31 +ellps=GRS80` à `2 49`) | Écart d'arrondi flottant, dépendant de la machine. PROJ le signale lui-même dans `test_proj.yaml` (`sub: ["8.53878e-07", "0"]`), Ce cas n'est dans aucun yaml du lot : aucun test du dépôt ne le compare. |
| `proj -b`, `-r`, `-s`, `-d`, `-e`, `-i`, `-l`, `-m`, `-o`, `-V` | Hors périmètre : options de `proj.cpp` non portées ; elles répondent `invalid option`. Pas de yaml du lot qui les utilise. |
