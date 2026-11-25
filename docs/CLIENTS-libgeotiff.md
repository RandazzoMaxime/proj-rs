# libgeotiff contre la bibliothèque C de proj-rs

libgeotiff lit et écrit les clés de géoréférencement des fichiers GeoTIFF. Pour nommer les codes
EPSG, elle interroge `proj.db` par l'API C de PROJ. Pour afficher les coins d'une image en
longitude et latitude, elle construit une chaîne PROJ et la transforme. GDAL l'embarque, mais
elle s'utilise aussi seule, par ses outils `listgeo` et `geotifcp`.

## Méthode

`tools/clients/libgeotiff.sh` clone libgeotiff au tag `1.7.4` (commit épinglé) et la compile une
fois contre PROJ 9.9.0, depuis la référence de `tools/build-reference.sh`.

Il lance ensuite, sur les 48 GeoTIFF de `test/data` :

- les deux tests de libgeotiff, `testlistgeo` et `testgeotifcp` ;
- `listgeo -proj4` sur chacun des 48 fichiers, coins compris ;

avec PROJ 9.9.0, puis avec notre `libproj`, chargée à sa place par `LD_LIBRARY_PATH` (même
soname, RUNPATH). Les deux passes utilisent le même dossier de données. Le script compare
directement les sorties des deux passes, et non au fichier attendu de libgeotiff : ce fichier a
été écrit avec une base EPSG plus ancienne (noms comme `ETRS89 / Belgian Lambert 2008` au lieu de
`ETRS89-BEL [BEREF2002] / Belgian Lambert 2008`), et PROJ 9.9.0 en diffère autant que nous.

### Les deux fichiers sans coins dans `testlistgeo`

`testlistgeo` n'imprime les coins que de 46 fichiers sur 48. Ce n'est ni PROJ ni
l'environnement : le script de test lui-même passe `-no_corners` à `listgeo` pour les deux
fichiers en Cassini-Soldner (`test/testlistgeo`, lignes 62 et 66) :

    $EXE -no_corners ${DATA_DIR}/ProjectedCSTypeGeoKey_28191_cassini_soldner.tif >>${OUT}
    $EXE -no_corners ${DATA_DIR}/cassini_soldner.tif >>${OUT}

Le fichier attendu de libgeotiff (`testlistgeo_out.dist`) n'a pas non plus leurs coins. Avec
PROJ 9.9.0, `listgeo -proj4` les calcule sans erreur, par exemple pour
`ProjectedCSTypeGeoKey_28191_cassini_soldner.tif` :

    PROJ.4 Definition: +proj=cass +lat_0=31.734096944 +lon_0=35.212080556 +x_0=170251.555 +y_0=126867.909 +a=6378300.789 +b=6356566.435 +units=m

    Corner Coordinates:
    Upper Left    (  440720.000, 3751320.000)  ( 40d47'28.08"E, 64d13'29.56"N)
    ...
    Center        (  440750.000, 3751290.000)  ( 40d47'30.10"E, 64d13'28.52"N)

Le point est à 3 600 km au nord de l'origine de la projection, loin du domaine où Cassini-Soldner
est utilisable, ce qui explique que libgeotiff ne fige pas ces valeurs. C'est pourquoi le script
ajoute la passe `listgeo -proj4` sur les 48 fichiers : les coins de ces deux fichiers y sont
comparés aussi, et proj-rs écrit exactement les mêmes.

### Quelle libproj chaque passe charge

`tools/clients/libproj_proof.sh` le prouve, pour chaque passe : `ldd` résout `libproj.so.25` de
`listgeo`, `geotifcp` et `libgeotiff.so.5` vers le fichier de la passe, sans `DT_RPATH` qui
mènerait à une autre ; `listgeo byte.tif` sous `LD_DEBUG=libs` n'initialise que ce fichier ; et
l'intercepteur `tools/clients/proj_count.c`, préchargé pendant toute la passe, vérifie que chaque
processus a chargé ce fichier et compte ses appels.

## Résultat (libgeotiff 1.7.4, libtiff 4.5.1, le 2026-11-25)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Fichiers listés par `testlistgeo` | 48 | 48 |
| ... avec leurs coins (`-no_corners` sur 2) | 46 | 46 |
| Fichiers avec leurs coins par `listgeo -proj4` | 48 | 48 |
| Lignes de `listgeo`, `listgeo -proj4` et `geotifcp` qui diffèrent | | 0 |

Preuves de la dernière passe :

    ref: ldd listgeo: libproj.so.25 => .../libgeotiff/ref-prefix/lib/libproj.so.25
    ref: ldd geotifcp: libproj.so.25 => .../libgeotiff/ref-prefix/lib/libproj.so.25
    ref: ldd libgeotiff.so.5: libproj.so.25 => .../libgeotiff/ref-prefix/lib/libproj.so.25
    ref: no DT_RPATH that could load another libproj
    ref: listgeo: Center        (  441320.000, 3750720.000)  (117d38' 4.70"W, 33d53'49.11"N)
    ref: LD_DEBUG=libs: calling init: .../libgeotiff/ref-prefix/lib/libproj.so.25
    ref: interposer: .../libgeotiff/ref-prefix/lib/libproj.so.25.9.9.0, proj_create_crs_to_crs=5/0 proj_trans=5/5
    ref: interposer: 107 processes mapped a libproj at start, 100 called into it, all .../libgeotiff/ref-prefix/lib/libproj.so.25.9.9.0
    ours: ldd listgeo: libproj.so.25 => .../libgeotiff/ours/lib/libproj.so.25
    ours: ldd geotifcp: libproj.so.25 => .../libgeotiff/ours/lib/libproj.so.25
    ours: ldd libgeotiff.so.5: libproj.so.25 => .../libgeotiff/ours/lib/libproj.so.25
    ours: no DT_RPATH that could load another libproj
    ours: listgeo: Center        (  441320.000, 3750720.000)  (117d38' 4.70"W, 33d53'49.11"N)
    ours: LD_DEBUG=libs: calling init: .../libgeotiff/ours/lib/libproj.so.25
    ours: interposer: .../libgeotiff/ours/lib/libproj.so.25, proj_create_crs_to_crs=5/0 proj_trans=5/5
    ours: interposer: 107 processes mapped a libproj at start, 100 called into it, all .../libgeotiff/ours/lib/libproj.so.25

## Écart trouvé et corrigé

Au premier passage du banc, deux fichiers en stéréographique polaire variante A (EPSG:5482, Ross
Sea Polar Stereographic 2000) n'avaient pas leurs coins avec proj-rs. libgeotiff écrit pour eux :

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
