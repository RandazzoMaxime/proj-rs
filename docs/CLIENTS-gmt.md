# GMT contre la bibliothèque C de proj-rs

GMT (Generic Mapping Tools) fait des cartes : projections, grilles, tracés PostScript. Sa suite de
tests est l'une des plus grosses du monde géospatial : 1240 scripts `ctest` (tests, scripts de la
documentation, exemples), chacun rend une image comparée à une image de référence.

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

`tools/clients/gmt.sh` clone GMT au tag `6.5.0` (commit épinglé), le compile une fois et
l'installe. Les tests tournent comme dans la CI de GMT (`.github/workflows/tests.yml`), pour qu'un
échec dise quelque chose de libproj :

- **les paquets** de `ci/install-dependencies-linux.sh` (`RUN_TESTS=true`, optionnels compris) :
  NetCDF, GDAL, FFTW, PCRE, LAPACK, GLib, GraphicsMagick, Ghostscript, LaTeX
  (`texlive-latex-base`, `texlive-binaries`, `texlive-fonts-recommended`), et `gawk`, que les
  scripts supposent (`gspline_7.sh` écrit une chaîne avec `%.1lf`, que `mawk` refuse). Le script
  les installe quand il tourne en root sous Debian ou Ubuntu ;
- **la configuration** de `ci/config-gmt-unix.sh` : fils et OpenMP, les options longues en essai
  (`USE_COMMON_LONG_OPTIONS`, `USE_MODULE_LONG_OPTIONS` : sans elles, les 46 tests `*-l2s.sh`
  échouent), le serveur de données `static` (les jeux figés pour les tests), les exemples, et
  `enable_testing()`, que GMT n'appelle jamais lui-même. Une différence : une construction
  Release, quand la CI construit en Debug avec `--coverage`. Compilé en Debug, `psxy` écrit la
  couleur de ses symboles `x` et `+` depuis une mémoire jamais initialisée
  (`1.75e-152 9.06e+223 2.72e-109 C` dans `comparesymbols.ps`, `vec_choice.ps`,
  `plot_symbols.ps`, `time_testing_5.ps`), que Ghostscript refuse (`/limitcheck`) : 4 tests
  échouent alors des deux côtés, qui passent en Release ;
- **les images de référence** : le dépôt DVC de GMT (DAGsHub) refuse la lecture anonyme (HTTP
  401). Chaque release en publie la copie, `baseline-images.zip` (le workflow
  `release-baseline-images.yml` fait un `dvc pull` du tag). Le script télécharge celle de 6.5.0,
  l'épingle par son sha256, et `tools/clients/gmt_baselines.py` vérifie chaque sortie contre le
  md5 que nomme son fichier `.dvc` (le md5 du fichier, ou celui de la liste du répertoire pour un
  `.dir`) avant de la poser là où `gmtest` la lit : 352 fichiers `.dvc` sur 352, 1101 images ;
- **les données distantes** : les 37 jeux de `ci-caches.yml` (le cache dont part la CI de GMT)
  et les fichiers du cache (`gmt get -Dcache`) sont téléchargés une fois, du serveur `static`,
  dans un répertoire personnel témoin, avant les passes. Avec un serveur fantôme comme `static`,
  GMT 6.5.0 range ses données sous `$HOME/.gmt/static` quel que soit `GMT_USERDIR`
  (`gmtremote_init_paths`), et `gmt get` y cherche sa table de hachage sous `server/`, où
  `gmt which` ne l'a pas écrite : le script l'y copie, sans quoi `gmt get -Dcache` ne télécharge
  rien. Chaque passe, et chaque relance isolée, part d'une copie de ce témoin au même chemin
  (`HOME`). Sans lui, quatre tests parallèles qui téléchargent le même fichier en même temps en
  lisent une moitié (une tuile JPEG 2000 tronquée, un `mississippi.txt` coupé) ;
- **l'installation** de GMT en tête du `PATH` : les scripts des suppléments (`fzmodeler`...) en
  viennent ;
- `RUNNER_OS=Linux`, que `gmtest` lit, comme sur GitHub Actions, pour prendre les marques
  `GMT_KNOWN_FAILURE_LINUX` des scripts (`met_uv.sh`) pour des échecs connus ;
- `ctest --output-on-failure --force-new-ctest-process -j4 --timeout 480`, puis deux fois
  `--rerun-failed`, comme la CI (`ctest || ctest --rerun-failed || ctest --rerun-failed`) :
  l'issue d'un test est celle de sa dernière exécution, et les tests qu'une relance a changés
  sont listés.

La suite tourne deux fois : une fois avec la libproj de PROJ 9.9.0 (la référence de
`tools/build-reference.sh`), une fois avec la nôtre (`capi/tests/dist.sh --release`), chacune en
tête de `LD_LIBRARY_PATH`.

### Quelle libproj chaque passe charge

Les binaires de GMT ne lient pas libproj ; ils lient le GDAL du système, qui lie `libproj.so.25`,
le soname de PROJ 9.9.0 comme de la bibliothèque de proj-rs. Rien d'autre que
`LD_LIBRARY_PATH` ne fait l'échange : deux passes sur la même bibliothèque donneraient « 0
différence » à tort. `gmt.sh` le vérifie donc avant et pendant chaque passe, et échoue sinon :

1. `ldd` du binaire `gmt`, avec le `LD_LIBRARY_PATH` de la passe, résout `libproj.so.25` vers le
   fichier de la passe ;
2. aucun objet de la chaîne (`gmt`, `libgmt`, `libgdal`...) ne porte de `DT_RPATH` qui serait
   cherché avant `LD_LIBRARY_PATH` : GMT est lié avec `-Wl,--enable-new-dtags`, et son chemin de
   construction est un `DT_RUNPATH` ;
3. un vrai `gmt mapproject -J+proj=utm` (qui atteint `proj_trans`) tourne sous `LD_DEBUG=libs`,
   et le `calling init:` de libproj doit être le fichier de la passe ;
4. pendant tout le `ctest`, relances comprises, l'intercepteur `tools/clients/proj_count.c`
   (chargé par `LD_PRELOAD` dans chaque processus) écrit, pour chaque processus qui a une libproj
   en mémoire, le chemin réel de celle-ci (`dl_iterate_phdr`) : un seul chemin étranger fait
   échouer la passe, et moins de 2000 processus `gmt` journalisés aussi.

Les relances isolées de `gmt_compare.py` passent par le même intercepteur, et leurs journaux sont
vérifiés de la même façon.

### Le comptage des appels à libproj

`proj_count.c` est un petit `.c` d'outillage, hors des crates (`tests/no_unsafe` ne lit que les
`*.rs`). Il transmet chaque appel tel quel à la fonction suivante (`dlsym(RTLD_NEXT)`, ou, pour une
libproj chargée avec `RTLD_LOCAL`, par son chemin) et compte, par processus, les appels et les
points de `proj_create`, `proj_create_argv`, `proj_create_crs_to_crs`,
`proj_create_crs_to_crs_from_pj`, `proj_create_from_wkt`, `proj_create_from_database`,
`proj_create_operations`, `proj_trans`, `proj_trans_array`, `proj_trans_generic`,
`proj_trans_bounds` et `proj_trans_bounds_3D`. Seuls comptent les appels du client : PROJ 9.9.0
appelle ses propres fonctions exportées par la PLT, proj-rs non, et un appel fait pendant un autre
appel compté du même fil n'est pas compté, pour que les deux passes se comparent.

### La comparaison

`tools/clients/gmt_compare.py` compare alors :

1. le verdict de chaque entrée ctest : il en faut autant que `ctest -N` en liste, dans les deux
   journaux, et le même des deux côtés ;
2. chaque fichier écrit par chaque test, récursivement. `gmtest`, le harnais de GMT, détruit le
   répertoire d'un test qui réussit : la copie de `gmtest` du répertoire de construction est
   modifiée pour le garder (son verdict ne change pas), si bien que les fichiers des tests
   réussis sont comparés aussi. Un répertoire présent d'un seul côté est une différence ;
   - octet par octet ; à défaut, normalisé : les PostScript sans leur ligne `%%CreationDate` ni
     les noms de fichiers temporaires de leurs commentaires `%@GMT:`, les netCDF par leur `ncdump`
     sans l'octet de la variable `grid_mapping` (un `char` scalaire que GMT n'initialise jamais) ;
   - un PostScript dont les octets diffèrent encore est rendu à 200 dpi par GraphicsMagick et
     comparé avec la limite RMS 0.003 de `gmtest` ; sous la limite, il n'échoue pas mais est
     compté et listé à part ;
3. tout ce qui diffère encore, ou qu'une seule passe a écrit, est relancé **isolément**, une fois
   avec chaque bibliothèque, et la paire fraîche est comparée avec la même règle. Si la paire
   fraîche ne s'accorde pas, le test est relancé une deuxième fois avec la référence : si les
   deux exécutions de référence diffèrent, GMT n'est pas déterministe là et aucun verdict sur la
   bibliothèque n'est possible ; sinon, une deuxième avec proj-rs : si elle diffère de la première
   et que le test n'appelle jamais libproj, c'est encore GMT ; sinon la différence est réelle et
   fait échouer la comparaison. Une relance qui n'écrit pas le fichier ne prouve rien : elle
   compte comme une différence ;
4. les appels à libproj de chaque test : mêmes nombres d'appels et de points par fonction dans
   les deux passes.

Il faut au moins 1100 tests et 3000 fichiers comparés, sinon la comparaison échoue. Ne sont pas
comparés : les PDF (Ghostscript y écrit sa date ; ils sont convertis des PostScript, qui le sont),
`gmt.history` et `*.tag` (horodatage) et les scripts eux-mêmes.

## Résultat (le 2026-11-25)

Machine : Linux x86-64 (Ubuntu 24.04), 4 vCPU, 15 Go de RAM. GMT 6.5.0 (commit `6da8ee2d`),
Release, GDAL 3.8.4 d'Ubuntu (`libgdal.so.34.3.8.4`), GSHHG 2.3.7, DCW 2.2.0, Ghostscript 10.02.1,
GraphicsMagick 1.3.42, images de référence de la release 6.5.0, données du serveur `static`.
proj-rs au commit `07edeec` de `main`. La libproj 9.4.0 du système n'est chargée par aucune passe.
Exécution complète de `gmt.sh` : 58 minutes (construction, téléchargement du témoin, deux passes
`ctest -j4` de 1224 et 1271 s relances comprises, relances isolées).

Bibliothèque chargée, vérifiée dans chaque passe (sortie du script) :

```
ref: ldd gmt: libproj.so.25 => /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25
ref: no DT_RPATH that could load another libproj (gmt: RUNPATH)
ref: gmt mapproject -J+proj=utm: 3 45 -> 500000	4982950.40023
ref: LD_DEBUG=libs: calling init: /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25
ref: interposer: /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25.9.9.0 mapped, proj_create=8/0 proj_create_crs_to_crs_from_pj=2/0 proj_trans=5/5
ref: 18384 processes mapped a libproj (18332 of them gmt), all /home/user/gmt-work/run/ref-prefix/lib/libproj.so.25.9.9.0
ours: ldd gmt: libproj.so.25 => /home/user/gmt-work/run/ours/lib/libproj.so.25
ours: no DT_RPATH that could load another libproj (gmt: RUNPATH)
ours: gmt mapproject -J+proj=utm: 3 45 -> 500000	4982950.40023
ours: LD_DEBUG=libs: calling init: /home/user/gmt-work/run/ours/lib/libproj.so.25
ours: interposer: /home/user/gmt-work/run/ours/lib/libproj.so.25 mapped, proj_create=8/0 proj_create_crs_to_crs_from_pj=2/0 proj_trans=5/5
ours: 18420 processes mapped a libproj (18368 of them gmt), all /home/user/gmt-work/run/ours/lib/libproj.so.25
reruns ref: 821 processes mapped a libproj, all .../ref-prefix/lib/libproj.so.25.9.9.0
reruns ours: 700 processes mapped a libproj, all .../ours/lib/libproj.so.25
```

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Tests (1240 listés, 1240 lus de chaque côté ; `ex53` désactivé par GMT) | 1240 | 1240 |
| Réussis | 1230 | 1228 |
| Échoués | 10 | 12 |
| dont échecs de la première exécution rattrapés par `--rerun-failed` | 17 | 12 |
| Échecs connus de GMT (`GMT_KNOWN_FAILURE`, comptés réussis) | 15 | 15 |
| Fichiers comparés (dont PostScript) | 5900 (1103) | 5900 (1103) |
| Répertoires de test d'un seul côté | 0 | 0 |
| Appels à libproj | 18 tests, 63 processus | 18 tests, 63 processus |

**Aucune différence réelle entre PROJ 9.9.0 et proj-rs** : `gmt_compare.py` conclut « every test
has the same outcome, every file the same content and every test the same libproj calls with both
libraries ». Les 4 tests dont l'issue diffère entre les deux passes n'appellent pas libproj, et
leur différence se défait ou se reproduit avec une seule bibliothèque :

- `potential/growth_o` et `psmask/masking` échouent dans la passe proj-rs (y compris ses deux
  relances parallèles) et réussissent dans la passe de référence ; relancés seuls, ils réussissent
  avec les deux bibliothèques, octet pour octet (GMT sous charge) ;
- `psxy/gaps` et `psxy/linetrim` : deux exécutions isolées avec la même bibliothèque donnent déjà
  des issues différentes (plus bas).

`grdproject/units` (`c.nc`, `i.nc`), la différence du banc précédent, est identique octet pour
octet : le `spatial_ref` de la grille vaut
`DATUM["unknown", SPHEROID["unknown",6378137,298.257220143428]]` avec les deux bibliothèques,
celui que PROJ 9.9.0 construit pour `+a=6378137.000 +b=6356752.314 +ellps=WGS84 +datum=WGS84`
(#361).

Appels à libproj (intercepteur, mêmes nombres dans les deux passes) : `proj_create` 274,
`proj_create_crs_to_crs_from_pj` 89, `proj_create_from_database` 38, `proj_create_from_wkt` 105,
`proj_trans` 28415 appels d'un point ; aucun `proj_trans_generic`, `proj_trans_array`,
`proj_trans_bounds` ni `proj_create_crs_to_crs` : GDAL 3.8 transforme point par point. Ils viennent
de 18 tests : `grdimage/grdimage_img_tif` (26331 `proj_trans`, 45 WKT lus), `mapproject/proj4`
(2084 `proj_trans`), les autres `grdimage` (`transp_mix`, `marbles`, `image_vartrans`,
`noJshiftimg`, `imgshade`), `grdmix/grid2img`, `grdview/texture2_classic` et `_modern`,
`grdproject/units`, `img/imgmap`, `img/imgtrack`, `doc/examples/ex52`, `doc/scripts/GMT_mixing`,
`gmtmex/WL_example_2` et `_3`, `psxy/categorical`. Les jeux distants étant téléchargés une fois
dans le témoin avant les passes, leur conversion par GDAL (le WKT des tuiles JPEG 2000) n'est plus
faite dans chaque passe : d'où 105 `proj_create_from_wkt` au lieu des 173 du banc précédent.

### Les 10 échecs de la référence, tous aussi échecs avec proj-rs

Aucun de ces tests n'appelle libproj (intercepteur), et leurs fichiers sont identiques dans les
deux passes, sauf ceux des deux derniers points.

| Test | Cause | Preuve |
|---|---|---|
| `grdinterpolate/slices`, `slices_file` | le fichier distant n'existe plus | le script fait `curl -s http://ds.iris.edu/spudservice/data/17996723 -o S362ANI_kmps.nc` ; IRIS répond 301 vers `https://...`, qui répond 404 : `curl` écrit 167 octets de HTML |
| `filter1d/mfilter` | le fichier distant n'existe plus | `filter1d [ERROR]: Libcurl Error: HTTP response code said error` sur `https://scrippsco2.ucsd.edu/assets/data/atmospheric/stations/in_situ_co2/weekly/weekly_in_situ_co2_mlo.csv` (404 au `curl` direct) |
| `psxy/urlquakes` | données vivantes | la requête `earthquake.usgs.gov/fdsnws/event/1/query.csv` (janvier 2016, M ≥ 4.5) rend le catalogue révisé de l'USGS : RMS 0.0035 contre 0.003 |
| `greenspline/gspline_7` | le script suppose le `grep` de macOS | `grep "^${X}\t"` : GNU grep 3.11 ne lit pas `\t` comme une tabulation (`printf -- '-1\t3\n' \| grep '^-1\t'` ne trouve rien), `Y` reste vide, puis `gmtmath [ERROR]: Operation "NEG" requires 1 operands` |
| `grdvector/shrink` | image | RMS 0.0042 contre 0.003, déterministe (même PostScript dans les deux passes) |
| `grdtrack/crosstrack_geo` | image | RMS 0.0172 contre 0.003, déterministe |
| `pscontour/thickness` | image | RMS 0.0206 contre la limite 0.02 du script, déterministe |
| `psxy/linearrow` | GMT non déterministe | `psxy [ERROR]: Found no history for option -R` (plus bas) |
| `psxy/linetrim` | GMT non déterministe | idem ; il réussit dans la passe proj-rs, échoue dans la référence, et deux exécutions isolées avec proj-rs donnent des issues différentes |

Pour `shrink`, `crosstrack_geo` et `thickness`, l'image diffère de la référence de GMT de la même
façon avec les deux bibliothèques, sans appel à libproj : la cause est dans le rendu ou la chaîne
de compilation, par rapport aux machines qui ont produit les références (la CI de GMT rend avec
le Ghostscript 10.02.0 de conda-forge, ce banc avec le 10.02.1 d'Ubuntu). Elle n'est pas
vérifiée plus avant.

### GMT n'est pas déterministe en 18 fichiers

Deux exécutions isolées avec la même bibliothèque y diffèrent déjà (`gmt_compare.py`, « two
reference runs: differs ») ; aucun verdict sur la bibliothèque n'est possible, et aucun de ces
tests n'appelle libproj :

- `byteswap/bswap16` et `bswap32` (10 fichiers) : leurs données viennent de `/dev/urandom` ;
- `psxy/gaps` (5 fichiers : `gaps_ascii.ps` au RMS 0.1136 entre deux exécutions de référence,
  `gaps_netcdf.ps` écrit par une exécution et pas par l'autre, leurs `_diff.png`, `gmt.conf`) ;
- `psxy/linearrow/linearrow.ps` (RMS 0.1329 entre deux exécutions de référence) et
  `psxy/linetrim` (`linetrim.ps`, RMS 0.0589 entre deux exécutions de référence, et
  `linetrim_diff.png`).

Pour `linearrow`, `linetrim` et `gaps`, la cause est dans GMT : `gmt math ... | gmt psxy -R -J`
lance les deux modules en même temps, et `gmtinit_put_history` (`gmt_init.c`) ouvre `gmt.history`
par `fopen (hfile, "w")`, ce qui le vide, avant d'en prendre le verrou `fcntl` ; le `psxy` qui lit
l'historique à ce moment ne trouve pas `-R` (`Found no history for option -R`) et la couche
manque.

Sont aussi listés à part, sans échec : 18 PostScript aux octets différents dans les passes mais
identiques dans la paire isolée, hors date et noms temporaires (RMS 0, sauf
`psevents_action.ps` à 0.0021), et 6 fichiers que la paire isolée rend identiques (le bruit de
GMT sous charge : `growth_o`, `masking`, `mgd77/convert/12345678.nc`, `psevents_action_diff.png`).

### Ce que le banc ne mesurait pas avant

La version précédente de ce document (1180 tests, 130 réussis, 1050 échoués des deux côtés)
comparait les deux passes sans images de référence (le dépôt DVC répondait 404), sans les
options longues ni les exemples, avec les données du serveur courant au lieu du serveur `static`
(SRTM15 v2.7, plus récent que les références : RMS 0.02 à 0.07 sur `genper`, `grdcontour`,
`grdfill`...), sans les scripts des suppléments sur le `PATH`, avec `mawk`, et sans relance des
échecs. Chaque passe partait d'un `GMT_USERDIR` vide, que le serveur `static` ignore ; ses tests
téléchargeaient en parallèle les mêmes fichiers et en lisaient parfois une moitié. Ces causes sont
réglées ci-dessus ; il reste les 10 échecs du tableau, communs aux deux bibliothèques.
