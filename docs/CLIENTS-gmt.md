# GMT contre la bibliothèque C de proj-rs

GMT (Generic Mapping Tools) fait des cartes : projections, grilles, tracés PostScript. Sa suite de
tests est l'une des plus grosses du monde géospatial : 1180 scripts `ctest`, chacun rend une image.

GMT n'appelle pas libproj lui-même : ses projections de tracé sont les siennes (`gmt_proj.c`). Il
l'atteint à travers GDAL, qu'il lie et consulte pour trois choses :

- `map_init_proj4` (`src/gmt_map.c`) : une projection donnée en syntaxe `-J+proj=...` délègue ses
  transformations avant et arrière à `OGRCreateCoordinateTransformation` de GDAL. Tous les
  `mapproject`, `grdproject`, `psxy` d'un script sur cette syntaxe passent par libproj, point par
  point ;
- `gmt_ogrproj.c` : reprojette des grilles ou vecteurs lus ou écrits avec un autre SRS ;
- `gmt_gdalread.c` : calcule les coins d'une image en longitude-latitude avec
  `OCTNewCoordinateTransformation`.

S'y ajoute, sans que GMT le demande, ce que GDAL fait de libproj en lisant et en écrivant les
fichiers : le WKT d'un GeoTIFF ou d'un JPEG 2000 relu (`proj_create_from_wkt`), le SRS d'une
grille écrite (`gmt_export2proj4` donne une chaîne PROJ, que GDAL convertit en WKT pour l'attribut
`spatial_ref` du netCDF).

## Méthode

`tools/clients/gmt.sh` clone GMT au tag `6.5.0` (commit épinglé) et le compile une fois. GMT exige
NetCDF, GDAL et GraphicsMagick (et Ghostscript pour rendre les PostScript) ; ses données GSHHG
2.3.7 et DCW 2.2.0 sont téléchargées de leurs releases GitHub et vérifiées par leur sha256 (les
mêmes que les recettes conda-forge). GMT n'appelle jamais `enable_testing()`, même quand
`DO_TESTS=ON` : le script le fournit par un `CMAKE_PROJECT_INCLUDE` d'une ligne, sans toucher
aux sources.

La suite tourne deux fois, avec les tests API et supplements activés (`DO_API_TESTS=ON`,
`DO_SUPPLEMENT_TESTS=ON`) : une fois avec la libproj de PROJ 9.9.0 (la référence de
`tools/build-reference.sh`), une fois avec la nôtre (`capi/tests/dist.sh --release`), chacune en
tête de `LD_LIBRARY_PATH`. Chaque passe part d'un répertoire utilisateur GMT vide
(`GMT_USERDIR`) : les tests qui utilisent des données distantes (`@earth_relief_...`) les
téléchargent du serveur de GMT dans chaque passe, et GDAL les décode (JPEG 2000, avec leur WKT)
dans chaque passe. Un cache commun laissait la seconde passe sans ces appels à libproj.

### Quelle libproj chaque passe charge

Les binaires de GMT ne lient pas libproj ; ils lient le GDAL du système, qui lie `libproj.so.25`,
le soname de PROJ 9.9.0 comme de la bibliothèque de proj-rs. Rien d'autre que
`LD_LIBRARY_PATH` ne fait l'échange : deux passes sur la même bibliothèque donneraient « 0
différence » à tort. `gmt.sh` le vérifie donc avant et pendant chaque passe, et échoue sinon :

1. `ldd` du binaire `gmt`, avec le `LD_LIBRARY_PATH` de la passe, résout `libproj.so.25` vers le
   fichier de la passe ;
2. aucun objet de la chaîne (`gmt`, `libgmt`, `libgdal`...) ne porte de `DT_RPATH` qui serait
   cherché avant `LD_LIBRARY_PATH` : GMT est lié avec `-Wl,--enable-new-dtags`, comme dans
   `pdal.sh`, et son chemin de construction est un `DT_RUNPATH` ;
3. un vrai `gmt mapproject -J+proj=utm` (qui atteint `proj_trans`) tourne sous `LD_DEBUG=libs`,
   et le `calling init:` de libproj doit être le fichier de la passe ;
4. pendant tout le `ctest`, l'intercepteur `tools/clients/proj_count.c` (compilé par le script et
   chargé par `LD_PRELOAD` dans chaque processus) écrit, pour chaque processus qui a une libproj
   en mémoire, le chemin réel de celle-ci (`dl_iterate_phdr`) : un seul chemin étranger fait
   échouer la passe, et moins de 2000 processus `gmt` journalisés aussi.

Les relances isolées de `gmt_compare.py` (plus bas) passent par le même intercepteur, et leurs
journaux sont vérifiés de la même façon.

### Le comptage des appels à libproj

`proj_count.c` est un petit `.c` d'outillage, hors des crates (`tests/no_unsafe` ne lit que les
`*.rs`). Il transmet chaque appel tel quel à la fonction suivante (`dlsym(RTLD_NEXT)`) et compte,
par processus, les appels et les points de `proj_create`, `proj_create_argv`,
`proj_create_crs_to_crs`, `proj_create_crs_to_crs_from_pj`, `proj_create_from_wkt`,
`proj_create_from_database`, `proj_create_operations`, `proj_trans`, `proj_trans_array`,
`proj_trans_generic`, `proj_trans_bounds` et `proj_trans_bounds_3D`. Seuls comptent les appels du
client : PROJ 9.9.0 appelle ses propres fonctions exportées par la PLT, proj-rs non, et un appel
fait pendant un autre appel compté du même fil n'est pas compté, pour que les deux passes se
comparent.

### La comparaison

`tools/clients/gmt_compare.py` compare alors :

1. le verdict de chaque entrée ctest : il en faut autant que `ctest -N` en liste, dans les deux
   journaux, et le même des deux côtés ;
2. chaque fichier écrit par chaque test, récursivement. `gmtest`, le harnais de GMT, détruit le
   répertoire d'un test qui réussit : la copie de `gmtest` du répertoire de construction est
   modifiée pour le garder (son verdict ne change pas), si bien que les fichiers des tests
   réussis sont comparés aussi. Un répertoire présent d'un seul côté est une différence ;
   - octet par octet ; à défaut, normalisé : les PostScript sans leur ligne `%%CreationDate`,
     les netCDF par leur `ncdump` sans l'octet de la variable `grid_mapping` (un `char`
     scalaire que GMT n'initialise jamais : deux exécutions de référence y diffèrent) ;
   - un PostScript dont les octets diffèrent encore est rendu à 200 dpi par GraphicsMagick et
     comparé avec la limite RMS 0.003 de `gmtest` ; sous la limite, il n'échoue pas mais est
     compté et listé à part ;
3. tout ce qui diffère encore, ou qu'une seule passe a écrit, est relancé **isolément**, une fois
   avec chaque bibliothèque, et la paire fraîche est comparée avec la même règle : GMT n'écrit
   pas toujours les mêmes fichiers dans une passe parallèle chargée (un psxy au bout d'un tube
   peut sauter sa couche). Si la paire fraîche ne s'accorde pas, le test est relancé une
   deuxième fois avec la référence : si les deux exécutions de référence diffèrent, GMT n'est
   pas déterministe là et aucun verdict sur la bibliothèque n'est possible ; sinon, une deuxième
   avec proj-rs : si elle diffère de la première et que le test n'appelle jamais libproj, c'est
   encore GMT ; sinon la différence est réelle et fait échouer la comparaison. Une relance qui
   n'écrit pas le fichier ne prouve rien : elle compte comme une différence ;
4. les appels à libproj de chaque test : mêmes nombres d'appels et de points par fonction dans
   les deux passes.

Il faut au moins 1100 tests et 3000 fichiers comparés, sinon la comparaison échoue. Ne sont pas
comparés : les PDF (Ghostscript y écrit sa date ; ils sont convertis des PostScript, qui le sont),
`gmt.history` et `*.tag` (horodatage) et les scripts eux-mêmes.

## Résultat

Machine : Linux x86-64 (Ubuntu 24.04), 4 vCPU, 15 Go de RAM. GMT 6.5.0 (commit `6da8ee2d`),
GDAL 3.8.4 d'Ubuntu (`libgdal.so.34.3.8.4`), GSHHG 2.3.7, DCW 2.2.0, Ghostscript 10.02.1,
GraphicsMagick 1.3.42, avec le réseau (les données distantes de GMT sont téléchargées). La
libproj 9.4.0 du système n'est chargée par aucune passe. Exécution complète de `gmt.sh` :
80 minutes (construction de GMT, deux passes `ctest -j4` de 884 et 855 s, relances isolées).

Bibliothèque chargée, vérifiée dans chaque passe (sortie du script) :

```
ref: ldd gmt: libproj.so.25 => /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25
ref: no DT_RPATH that could load another libproj (gmt: RUNPATH)
ref: gmt mapproject -J+proj=utm: 3 45 -> 500000	4982950.40023
ref: LD_DEBUG=libs: calling init: /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25
ref: interposer: /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25.9.9.0 mapped, proj_create=8/0 proj_create_crs_to_crs_from_pj=2/0 proj_trans=5/5
ref: 16914 processes mapped a libproj (16862 of them gmt), all /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25.9.9.0
ours: ldd gmt: libproj.so.25 => /home/user/gmt-work/run/ours/lib/libproj.so.25
ours: no DT_RPATH that could load another libproj (gmt: RUNPATH)
ours: gmt mapproject -J+proj=utm: 3 45 -> 500000	4982950.40023
ours: LD_DEBUG=libs: calling init: /home/user/gmt-work/run/ours/lib/libproj.so.25
ours: interposer: /home/user/gmt-work/run/ours/lib/libproj.so.25 mapped, proj_create=8/0 proj_create_crs_to_crs_from_pj=2/0 proj_trans=5/5
ours: 16918 processes mapped a libproj (16866 of them gmt), all /home/user/gmt-work/run/ours/lib/libproj.so.25
reruns ref: 1023 processes mapped a libproj, all .../ref-prefix/lib/libproj.so.25.9.9.0
reruns ours: 708 processes mapped a libproj, all .../ours/lib/libproj.so.25
```

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Verdicts ctest (1180 listés, 1180 lus de chaque côté) | 130 réussis, 1050 échoués | idem, test par test |
| Fichiers comparés (dont PostScript) | 4673 (1040) | 4673 (1040) |
| Répertoires de test d'un seul côté | 0 | 0 |
| Appels à libproj | 29 tests, 74 processus | 29 tests, 75 processus |

Les 1050 échecs sont les mêmes des deux côtés : les baselines DVC sont absentes.

Fichiers qui diffèrent entre les deux passes complètes :

- **3 PostScript** aux octets différents mais au RMS 0 (listés à part, pas des échecs) :
  `doc/scripts/GMT_App_J_1`, `grdblend/center_cf_g` (la paire isolée est identique) et
  `grdblend/center_bf_p` (la paire isolée diffère encore d'octets, RMS 0) ;
- **34** fichiers que la paire isolée rend identiques : le bruit de GMT sous charge
  (`grdfill/*`, `grdfft/bworthxy`, `grdview/texture2_*`, `grdcut/origin`...) ;
- **18** fichiers où deux exécutions isolées avec la même bibliothèque diffèrent déjà : GMT n'y
  est pas déterministe, aucun verdict sur la bibliothèque n'est possible. Ce sont
  `byteswap/bswap16` et `bswap32` (10 fichiers : leurs données viennent de `/dev/urandom`), et 8
  fichiers de 7 tests : `filter1d/smooth_track`, `grdmath/reduce` (et son `tmp.grd`), `grdtrack/crosstrack_geo`,
  `psxy/linearrow`, `psxy/linetrim`, `sample1d/splines`, où deux exécutions de référence
  diffèrent, et `potential/growth_p`, où deux exécutions avec proj-rs diffèrent dans un test
  qui n'appelle jamais libproj ;
- **2 différences réelles**, un défaut de proj-rs (plus bas) : `grdproject/units/c.nc` et
  `i.nc`.

Appels à libproj (intercepteur, passe de référence) : `proj_create` 270,
`proj_create_crs_to_crs_from_pj` 87, `proj_create_from_database` 54, `proj_create_from_wkt`
173, `proj_trans` 28415 appels d'un point ; aucun `proj_trans_generic`, `proj_trans_array`,
`proj_trans_bounds` ni `proj_create_crs_to_crs` : GDAL 3.8 transforme point par point. Ils
viennent de 29 tests : `grdimage/grdimage_img_tif` (26331 `proj_trans`, 45 WKT lus),
`mapproject/proj4` (2084 `proj_trans`), les autres `grdimage` (`transp_mix`, `marbles`,
`image_vartrans`, `noJshiftimg`, `imgshade`, `twogrids`, `rounding`), `grdmix/grid2img`,
`grdview/texture2_*`, `grdproject/units`, `img/imgmap`, `doc/scripts/GMT_mixing`, et les tests
qui lisent un jeu distant (le WKT de ses tuiles JPEG 2000). Trois tests ont des comptes
différents entre les passes (`doc/scripts/GMT_tut_11`, `GMT_tut_18`,
`grdview/texture2_modern`) : dans une passe parallèle, c'est le premier test qui demande un jeu
distant qui le télécharge et le convertit ; rejoués seuls, chacun fait les mêmes appels avec les
deux bibliothèques. Le total diffère de 4 `proj_create_from_wkt` pour la même raison.

### Le défaut trouvé : `+datum=` passe devant `+a`/`+b`

Pour une grille projetée, GMT écrit son SRS en chaîne PROJ (`gmt_export2proj4`), avec à la fois
les demi-axes arrondis au millimètre et l'ellipsoïde et le datum nommés :
`+proj=merc ... +a=6378137.000 +b=6356752.314 +ellps=WGS84 +datum=WGS84 +units=m +no_defs`.
GDAL la convertit en WKT pour l'attribut `spatial_ref` du netCDF. PROJ 9.9.0 donne la priorité
aux nombres : `DATUM["unknown", SPHEROID["unknown",6378137,298.257220143428]]`. proj-rs garde le
datum nommé et jette `+a`/`+b` : `DATUM["WGS_1984", SPHEROID["WGS 84",6378137,298.257223563]]`.
Pour reproduire, avec la référence de `tools/build-reference.sh` et le `projinfo` de proj-rs :

```bash
S="+proj=longlat +a=6378137 +b=6356752.314 +datum=WGS84 +type=crs"
PROJ_DATA=target/proj-reference/build/data target/proj-reference/build/bin/projinfo -o WKT1_GDAL -q "$S"
cargo run --release --bin projinfo -- -o WKT1_GDAL -q "$S"
```

Sans `+datum` (`+a +b` seuls, ou avec `+ellps=WGS84`) les deux donnent l'ellipsoïde `unknown` ;
avec `+datum=WGS84` et `+rf=` au lieu de `+b`, même écart. Par GDAL, comme GMT :
`gdalsrsinfo -o wkt1 "<chaîne>"` avec chacune des deux bibliothèques en tête de
`LD_LIBRARY_PATH`. Ce défaut n'est pas corrigé ici : le banc le rapporte et échoue
(`FAILED: files`).

### Ce qui avait faussé la première version de ce banc

La version précédente concluait « aucune différence » sur 3974 fichiers. Elle ne prouvait pas
que chaque passe chargeait la bonne bibliothèque, ne comparait pas les fichiers des tests
réussis (supprimés par `gmtest`), acceptait un PostScript au seul RMS, sautait sans le dire un
répertoire présent d'un seul côté, et concluait à l'accord quand aucune relance n'écrivait le
fichier. Elle partageait aussi `~/.gmt` entre les passes : la première téléchargeait et
convertissait les données distantes, la seconde les trouvait en cache et ne faisait pas ces
appels à libproj. L'écart de `grdproject/units` n'y figurait pas ; il apparaît ici (GDAL 3.8.4
d'Ubuntu, quand la version précédente annonçait GDAL 3.10.3 de Debian) et se reproduit hors de
GMT, avec `projinfo`.
