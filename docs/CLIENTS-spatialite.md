# SpatiaLite contre la bibliothèque C de proj-rs

SpatiaLite est l'extension spatiale de SQLite. `ST_Transform`, les fonctions `PROJ_*` et la table
`spatial_ref_sys` passent par libproj, sans GDAL entre les deux. Sa suite de tests est un client de
plus, léger et rapide à rejouer.

## Méthode

`tools/clients/spatialite.sh` compile libspatialite 5.1.0 (source épinglée par son sha256) une
fois, contre PROJ 9.9.0 de la référence de `tools/build-reference.sh`. Le script vérifie qu'elle
utilise l'API PROJ 6+ (`PROJ_NEW`).

Il lance ensuite `make check` deux fois, hors réseau et avec le même dossier de données :

- avec PROJ 9.9.0 ;
- avec notre `libproj` (`capi/tests/dist.sh`), chargée à sa place par `LD_LIBRARY_PATH`. Le
  soname est le même (`libproj.so.25`), et libspatialite est liée en RUNPATH.

Il compare enfin le journal de chaque test, ligne par ligne.

## Résultat (libspatialite 5.1.0, le 2026-11-22)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 104 sur 108 | 104 sur 108 |
| Journaux de test différents | | 0 sur 108 |

Les 4 échecs sont les mêmes des deux côtés, au même cas et avec le même message :
`check_bufovflw`, `check_sql_stmt`, `check_sql_stmt_tiny` et `check_sql_stmt_extension`. Ils
viennent de la suite de SpatiaLite 5.1.0 face à PROJ 9.9.0, pas de proj-rs. Par exemple, le cas
`transform - Point (unknown srid)` attend un autre résultat que celui de PROJ 9.9.0.

## Écart trouvé et corrigé

Au premier passage, un seul journal différait. Pour un CRS inconnu (`NONE:0`),
`proj_create_crs_to_crs` renvoyait notre propre message. PROJ, lui, crée d'abord chaque CRS avec
`proj_create` (`crs_to_crs.cpp:339-350`) et laisse la ligne de ce refus :
`proj_create: crs not found: NONE:0`, code 4096. proj-rs fait désormais de même quand la paire
échoue (`capi/src/trans.rs`), et `capi/tests/errno_parity.c` couvre ce cas.
