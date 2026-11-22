# PDAL contre la bibliothèque C de proj-rs

PDAL traite les nuages de points (lidar, photogrammétrie). Il appelle libproj directement
(`find_package(PROJ)`) et aussi à travers GDAL, pour lire, écrire et reprojeter les systèmes de
coordonnées des fichiers LAS, LAZ, COPC ou EPT.

## Méthode

`tools/clients/pdal.sh` clone PDAL au tag `2.10.2` (commit épinglé). Il le compile une fois, avec
ses tests, contre PROJ 9.9.0 (la référence de `tools/build-reference.sh`) et contre le GDAL du
système (3.8 ou plus récent).

Il lance ensuite toute la suite `ctest` deux fois :

- avec PROJ 9.9.0 ;
- avec notre `libproj`, chargée à sa place par `LD_LIBRARY_PATH` (même soname). PDAL et GDAL
  passent alors tous deux par notre bibliothèque.

Les tests tournent un par un (`-j1`) : `pdal_io_bpf_base_test` et `pdal_io_bpf_zlib_test`
partagent des fichiers temporaires, et échouent parfois quand ils tournent en même temps, avec l'une
comme avec l'autre bibliothèque. Le script compare la liste des tests qui échouent.

## Résultat (PDAL 2.10.2 et GDAL 3.8.4, le 2026-11-22)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 140 sur 142 | 140 sur 142 |
| Tests qui échouent | `pdal_io_stac_reader_test`, `pdal_io_copc_remote_reader_test` | les mêmes |

Ces deux tests lisent des fichiers distants. Ils échouent hors réseau, avec l'une comme avec
l'autre bibliothèque.

## Écarts trouvés et corrigés

Au premier passage, `pdal_io_copc_writer_test` échouait avec proj-rs seulement, sur le cas
`CopcWriterTest.srsWkt2`. Son CRS est un `DERIVEDPROJCRS` dont la conversion a pour méthode
`PROJ-based operation method: +proj=pipeline ...`. Deux écarts s'enchaînaient :

1. **WKT pris pour une chaîne PROJ.** Le texte contient ` +proj=`, et proj-rs le lisait comme une
   chaîne PROJ, dans `proj_create` comme dans `create_user_object`. GDAL ne pouvait donc pas le
   charger. PROJ lit d'abord un texte qui commence par un mot-clé WKT suivi de `[` comme du WKT
   (`io.cpp:7780-7797`). proj-rs fait désormais de même (`iso19111::opens_as_wkt`).
2. **Avertissement manquant.** PROJ ajoute à `proj_create_from_wkt` l'avertissement
   `Unknown method PROJ-based operation method: ...` de `validateParameters`. GDAL ne met un CRS
   en cache que s'il n'a reçu aucun avertissement. Sans celui-ci, il prenait un autre chemin, qui
   finissait sur un export WKT1 impossible pour un `DerivedProjectedCRS`. proj-rs ajoute
   maintenant ces avertissements comme PROJ. Sur le WKT2 et le WKT1 de tous les CRS et opérations
   EPSG, ses avertissements sont ceux de libproj (`capi/tests/wkt_warnings.c`).

Les deux corrections sont détaillées dans `docs/PORTING-creation-refusal.md`.

La cause a été trouvée en traçant les appels de GDAL et de PDAL vers libproj (`ltrace`) dans les
deux passes : les traces se séparaient juste après `proj_create_from_wkt`, là où GDAL décide de
mettre le CRS en cache ou non.
