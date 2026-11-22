# libgeotiff contre la bibliothèque C de proj-rs

libgeotiff lit et écrit les clés de géoréférencement des fichiers GeoTIFF. Pour nommer les codes
EPSG, elle interroge `proj.db` par l'API C de PROJ. Pour afficher les coins d'une image en
longitude et latitude, elle construit une chaîne PROJ et la transforme. GDAL l'embarque, mais
elle s'utilise aussi seule, par ses outils `listgeo` et `geotifcp`.

## Méthode

`tools/clients/libgeotiff.sh` clone libgeotiff au tag `1.7.4` (commit épinglé) et la compile une
fois contre PROJ 9.9.0, depuis la référence de `tools/build-reference.sh`.

Il lance ensuite ses deux tests, `testlistgeo` et `testgeotifcp`, sur les 48 GeoTIFF de
`test/data` :

- avec PROJ 9.9.0 ;
- avec notre `libproj`, chargée à sa place par `LD_LIBRARY_PATH` (même soname, RUNPATH).

Les deux passes utilisent le même dossier de données. Le script compare directement les sorties des
deux passes, et non au fichier attendu de libgeotiff : ce fichier a été écrit avec une base EPSG
plus ancienne (noms comme `ETRS89 / Belgian Lambert 2008` au lieu de `ETRS89-BEL [BEREF2002] /
Belgian Lambert 2008`), et PROJ 9.9.0 en diffère autant que nous.

## Résultat (libgeotiff 1.7.4, le 2026-11-22)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Fichiers listés par `listgeo` | 48 | 48 |
| Fichiers avec leurs coins en longitude, latitude | 46 | 46 |
| Lignes de `listgeo` et `geotifcp` qui diffèrent | | 0 |

## Écart trouvé et corrigé

Au premier passage, deux fichiers en stéréographique polaire variante A (EPSG:5482, Ross Sea
Polar Stereographic 2000) n'avaient pas leurs coins. libgeotiff écrit pour eux :

    +proj=stere +lat_0=-90 +lat_ts=-90 +lon_0=180 +k=0.994 +x_0=5000000 +y_0=1000000 +ellps=GRS80

proj-rs refusait toute stéréographique polaire qui donne à la fois `+lat_ts` et un `+k` différent
de 1. PROJ ne refuse que si `lat_ts` n'est pas aussi le pôle (`io.cpp:12111-12126`). Avec
`lat_ts` au pôle et `k ≠ 1`, la chaîne est la variante A, de facteur d'échelle `k`. Le refus porte
désormais le texte de PROJ : `lat_ts != lat_0 and k != 1 not supported for Polar Stereographic`.
Deux tests de `tests/test_io.rs` le vérifient, avec les valeurs de `projinfo` 9.9.0.

## Écarts vus en route, hors de ce lot

`projinfo` sur une chaîne PROJ, comparé à celui de PROJ 9.9.0 :

- les axes d'une stéréographique ou d'une `laea` polaire lues depuis une chaîne PROJ : PROJ les
  décrit par rapport à un méridien (`AXIS["(E)",north,MERIDIAN[90,...]]`), proj-rs écrit
  `east` et `north`. L'ordre des axes est le même, donc les coordonnées transformées aussi ;
- la ligne d'erreur quand la lecture échoue : PROJ écrit `input string: parsing of '...' failed:`
  avant le message, proj-rs écrit le message seul.
