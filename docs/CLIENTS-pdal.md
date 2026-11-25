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
comme avec l'autre bibliothèque. Le script compare la liste des tests qui échouent, et échoue si
elle diffère.

### Le réseau

Deux tests lisent des fichiers distants :

- `pdal_io_stac_reader_test` : un item STAC sur `s3-us-west-2.amazonaws.com`, et les schémas JSON
  de ses catalogues, en HTTP simple (`http://json-schema.org/draft-07/schema`) ;
- `pdal_io_copc_remote_reader_test` : `https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz`,
  directement et par le `/vsicurl/` de GDAL.

Le script vérifie d'abord que ces adresses répondent, et le dit quand l'une ne répond pas.

La couche HTTP de PDAL (arbiter) donne le proxy de `HTTPS_PROXY` aussi aux adresses `http://`.
Derrière un proxy qui n'accepte que `CONNECT` (pas de `HTTP_PROXY`), ses requêtes HTTP simples sont
refusées, et `StacReaderTest.schema_validate_test` échoue sur le schéma JSON. Dans ce cas, le
script lance `tools/clients/split_proxy.py` et le donne à arbiter (`CURL_PROXY`) : chaque
`CONNECT` part vers le proxy d'origine sans changement, chaque requête HTTP simple va directement à
son hôte, comme pour tout client qui a `HTTPS_PROXY` sans `HTTP_PROXY`.

### Quelle libproj chaque passe charge

`tools/clients/libproj_proof.sh` le prouve, pour chaque passe : `ldd` résout `libproj.so.25` de
`pdal` et de `libpdalcpp.so` vers le fichier de la passe, sans `DT_RPATH` qui mènerait à une
autre ; `pdal translate` avec `filters.reprojection` (EPSG:32617 vers EPSG:4326) sous
`LD_DEBUG=libs` n'initialise que ce fichier ; l'intercepteur `tools/clients/proj_count.c`,
préchargé pendant tout `ctest`, vérifie que chaque processus a chargé ce fichier et compte ses
appels.

## Résultat (PDAL 2.10.2, GDAL 3.8.4, libcurl 8.5.0, le 2026-11-25)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests réussis | 141 sur 142 | 141 sur 142 |
| Tests qui échouent | `pdal_io_copc_remote_reader_test` | le même |

Le seul échec vient du réseau de la machine du banc, pas de PDAL ni de libproj. Le proxy de
sortie de la session refuse cette adresse de `github.com` :

    C++ exception with description "Could not read from
    'github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz'. Response message
    returned '{"message":"GitHub access to this repository is not enabled for this session. ..."}'"
    thrown in the test body.
    [  FAILED  ] CopcRemoteReaderTest.vsi

Le script l'annonce avant la suite (`remote test file NOT reachable (HTTP 403)`). Sur une machine
qui atteint `github.com`, ce test lit le fichier ; il n'a pas tourné ici.

Preuves de la dernière passe :

    ref: ldd pdal: libproj.so.25 => .../pdal/ref-prefix/lib/libproj.so.25
    ref: ldd libpdalcpp.so: libproj.so.25 => .../pdal/ref-prefix/lib/libproj.so.25
    ref: no DT_RPATH that could load another libproj
    ref: LD_DEBUG=libs: calling init: .../pdal/ref-prefix/lib/libproj.so.25
    ref: interposer: .../pdal/ref-prefix/lib/libproj.so.25.9.9.0, proj_create=2/0 proj_create_crs_to_crs_from_pj=1/0 proj_create_from_wkt=3/0 proj_create_from_database=2/0 proj_trans=1065/1065
    ref: interposer: 248 processes mapped a libproj at start, 111 called into it, all .../pdal/ref-prefix/lib/libproj.so.25.9.9.0
    ours: ldd pdal: libproj.so.25 => .../pdal/ours/lib/libproj.so.25
    ours: ldd libpdalcpp.so: libproj.so.25 => .../pdal/ours/lib/libproj.so.25
    ours: no DT_RPATH that could load another libproj
    ours: LD_DEBUG=libs: calling init: .../pdal/ours/lib/libproj.so.25
    ours: interposer: .../pdal/ours/lib/libproj.so.25, proj_create=2/0 proj_create_crs_to_crs_from_pj=1/0 proj_create_from_wkt=3/0 proj_create_from_database=2/0 proj_trans=1065/1065
    ours: interposer: 248 processes mapped a libproj at start, 111 called into it, all .../pdal/ours/lib/libproj.so.25

## Écarts trouvés et corrigés

Au premier passage du banc, `pdal_io_copc_writer_test` échouait avec proj-rs seulement, sur le cas
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
