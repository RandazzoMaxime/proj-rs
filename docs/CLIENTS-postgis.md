# PostGIS contre la bibliothèque C de proj-rs

PostGIS est l'extension spatiale de PostgreSQL. `ST_Transform`, `postgis_proj_version`, la table
`spatial_ref_sys` et la topologie passent par libproj, chargée dans le processus du serveur
PostgreSQL lui-même.

## Méthode

`tools/clients/postgis.sh` télécharge PostGIS 3.6.4 (archive épinglée par son sha256), le compile
contre PROJ 9.9.0 (la référence de `tools/build-reference.sh`) et l'installe dans le PostgreSQL 16
du système. Il crée ensuite un cluster temporaire et lance `make check-regress` deux fois :

- serveur démarré avec PROJ 9.9.0 ;
- serveur démarré avec notre `libproj`, chargée à sa place par `LD_LIBRARY_PATH` (même soname,
  RUNPATH).

Le module `postgis-3.so` est chargé par le serveur, donc c'est l'environnement du serveur qui
choisit la bibliothèque. Dans la seconde passe, `/proc/<pid>/maps` d'un processus serveur montre
bien notre `libproj.so.25`. La partie raster est laissée de côté (`--without-raster`) : elle
atteint libproj à travers GDAL, qui a déjà sa propre comparaison.

Le script tourne en root, puisqu'il installe dans le PostgreSQL du système et démarre le serveur
sous l'utilisateur `postgres`.

## Résultat (PostGIS 3.6.4, PostgreSQL 16.15, GEOS 3.12.1, le 2026-11-22)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests de régression réussis | 247 sur 247 | 247 sur 247 |

Chaque test de PostGIS compare sa sortie à un fichier attendu : 247 tests réussis des deux côtés
veulent dire 247 sorties identiques au texte attendu. Aucun écart n'a été trouvé.

PostGIS 3.6 recommande GEOS 3.14 ; avec la 3.12.1 du système, la suite passe entièrement avec
l'une comme avec l'autre bibliothèque.
