# SpatiaLite contre la bibliothèque C de proj-rs

SpatiaLite est l'extension spatiale de SQLite. `ST_Transform`, les fonctions `PROJ_*` et la table
`spatial_ref_sys` passent par libproj, sans GDAL entre les deux. Sa suite de tests est un client de
plus, léger et rapide à rejouer.

## Méthode

`tools/clients/spatialite.sh` compile libspatialite 5.1.0 (source épinglée par son sha256) une
fois, avec ses programmes de test, contre PROJ 9.9.0 de la référence de
`tools/build-reference.sh`. Le script vérifie qu'elle utilise l'API PROJ 6+ (`PROJ_NEW`).

Il lance ensuite `make check` deux fois, hors réseau et avec le même dossier de données :

- avec PROJ 9.9.0 ;
- avec notre `libproj` (`capi/tests/dist.sh`), chargée à sa place par `LD_LIBRARY_PATH`. Le
  soname est le même (`libproj.so.25`), et libspatialite est liée en RUNPATH.

Il compare enfin le journal de chaque test, ligne par ligne, et échoue si un seul diffère.

### SQLite 3.42.0

libspatialite 5.1.0 (4 août 2023) est compilée contre SQLite 3.42.0, la version pour laquelle sa
suite a été écrite : le script la télécharge (sha256 épinglé) et la compile en bibliothèque
partagée, avec les options de celle de Debian dont SpatiaLite se sert (R*Tree, métadonnées de
colonnes, FTS, fonctions mathématiques...). Il vérifie que libspatialite la charge.

Avec SQLite 3.43.0 et suivantes, `check_bufovflw` échoue quelle que soit la libproj. Son cas
`AsKML#1` construit le texte attendu avec `sqlite3_mprintf("%1.0f", 1e128)`, et `AsKml` écrit la
coordonnée avec `%.300f` puis retire les zéros. À partir de SQLite 3.43.0, les deux ne donnent
plus les mêmes chiffres :

| SQLite | `printf('%1.0f', 1e128)` = `printf('%.300f', 1e128)` sans les zéros |
|---|---|
| 3.42.0 | oui |
| 3.43.0, 3.44.0, 3.45.0 | non (`100000000000000007500...` contre `100000000000000000000...`) |

### Quelle libproj chaque passe charge

`tools/clients/libproj_proof.sh` le prouve, pour chaque passe : `ldd` résout `libproj.so.25` de
`libspatialite.so` et `mod_spatialite.so` vers le fichier de la passe, sans `DT_RPATH` qui
mènerait à une autre ; `ST_Transform` dans le shell SQLite qui charge `mod_spatialite`, sous
`LD_DEBUG=libs`, n'initialise que ce fichier ; l'intercepteur `tools/clients/proj_count.c`,
préchargé pendant tout `make check`, vérifie que chacun des processus de test a chargé ce fichier
et compte ses appels.

## Résultat (libspatialite 5.1.0, SQLite 3.42.0, GEOS 3.12.1, le 2026-11-25)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 105 sur 108 | 105 sur 108 |
| Tests qui échouent | `check_sql_stmt`, `check_sql_stmt_tiny`, `check_sql_stmt_extension` | les mêmes |
| Journaux de test différents | | 0 sur 108 |

Les 3 échecs viennent de la suite de SpatiaLite 5.1.0, qui attend le message d'erreur de PROJ 9.1
à 9.4. Ils tombent au même cas, le 3 326ᵉ de chacun des trois programmes :
`sql_stmt_proj910_tests/transform4.testcase` (`transform - Point (unknown srid)`), que le programme
choisit pour toute PROJ 9.1 ou plus récente (`check_sql_stmt.h:711-718`). Il transforme un point
sans SRID et attend :

    ST_Transform exception - PROJ reports "proj_create: crs not found".

PROJ 9.9.0 écrit :

    ST_Transform exception - PROJ reports "proj_create: crs not found: NONE:0".

Depuis PROJ 9.5.0, `proj_create` ajoute le code demandé au message de
`NoSuchAuthorityCodeException` (`c_api.cpp:639-646` en 9.9.0 ; absent des sources de 9.1.0 à
9.4.0). proj-rs écrit exactement la même ligne.

Ces programmes s'arrêtent à leur premier cas en échec. Pour que les cas suivants tournent aussi, le
script les relance une fois de plus dans chaque passe, avec le message de PROJ 9.5 dans
`transform4.testcase` (le fichier est rétabli ensuite), et compare aussi ces journaux :

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| `check_sql_stmt`, avec le message de PROJ 9.5 | réussi, 7 241 cas | réussi, 7 241 cas |
| `check_sql_stmt_tiny`, idem | réussi, 7 241 cas | réussi, 7 241 cas |
| `check_sql_stmt_extension`, idem | réussi, 7 241 cas | réussi, 7 241 cas |

Preuves de la dernière passe :

    ref: ldd libspatialite.so: libproj.so.25 => .../proj-reference/build/lib/libproj.so.25
    ref: ldd mod_spatialite.so: libproj.so.25 => .../proj-reference/build/lib/libproj.so.25
    ref: no DT_RPATH that could load another libproj
    ref: sqlite3: POINT(500000 4982950.400227)
    ref: LD_DEBUG=libs: calling init: .../proj-reference/build/lib/libproj.so.25
    ref: interposer: .../proj-reference/build/lib/libproj.so.25.9.9.0, proj_create_crs_to_crs=1/0 proj_trans_generic=1/1
    ref: interposer: 111 processes mapped a libproj at start, 10 called into it, all .../proj-reference/build/lib/libproj.so.25.9.9.0
    ours: ldd libspatialite.so: libproj.so.25 => .../spatialite/ours/lib/libproj.so.25
    ours: ldd mod_spatialite.so: libproj.so.25 => .../spatialite/ours/lib/libproj.so.25
    ours: no DT_RPATH that could load another libproj
    ours: sqlite3: POINT(500000 4982950.400227)
    ours: LD_DEBUG=libs: calling init: .../spatialite/ours/lib/libproj.so.25
    ours: interposer: .../spatialite/ours/lib/libproj.so.25, proj_create_crs_to_crs=1/0 proj_trans_generic=1/1
    ours: interposer: 111 processes mapped a libproj at start, 10 called into it, all .../spatialite/ours/lib/libproj.so.25

## Écart trouvé et corrigé

Au premier passage du banc, un seul journal différait. Pour un CRS inconnu (`NONE:0`),
`proj_create_crs_to_crs` renvoyait notre propre message. PROJ, lui, crée d'abord chaque CRS avec
`proj_create` (`crs_to_crs.cpp:339-350`) et laisse la ligne de ce refus :
`proj_create: crs not found: NONE:0`, code 4096. proj-rs fait désormais de même quand la paire
échoue (`capi/src/trans.rs`), et `capi/tests/errno_parity.c` couvre ce cas.
