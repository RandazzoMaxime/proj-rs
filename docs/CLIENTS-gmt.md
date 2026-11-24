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

C'est donc le côté GDAL de libproj que GMT fait travailler, et pas celui des SIG déjà testés :
des chaînes PROJ construites par GDAL (`proj_create` puis transformation OCT), du WKT lu à
l'ouverture d'un GeoTIFF, un SRS réécrit en sortie.

## Méthode

`tools/clients/gmt.sh` clone GMT au tag `6.5.0` (commit épinglé) et le compile une fois. GMT exige
NetCDF, GDAL et GraphicsMagick (et Ghostscript pour rendre les PostScript), et ses données
GSHHG et DCW sont téléchargées de leurs releases GitHub. GMT n'appelle jamais
`enable_testing()`, même quand `DO_TESTS=ON` : le script le fournit par un `CMAKE_PROJECT_INCLUDE`
d'une ligne, sans toucher aux sources.

La suite tourne deux fois, avec les tests API et supplements activés (`DO_API_TESTS=ON`,
`DO_SUPPLEMENT_TESTS=ON`) :

- avec la libproj de PROJ 9.9.0 (la référence de `tools/build-reference.sh`) en tête de
  `LD_LIBRARY_PATH` (même soname que celle du proj de la distribution, que le GDAL du système a
  liée) ;
- avec notre `libproj` au même endroit.

Le script garde alors tout ce que chaque test a écrit (`outputs-ref/`, `outputs-ours/`) : le
harnais de GMT, `gmtest`, détruit le répertoire de travail d'un test qui réussit sa comparaison
contre une baseline, et pas celui d'un test qui échoue. Ces baselines sont dans un dépôt DVC qui
répond 404 aujourd'hui : `gmtest` ne peut pas comparer à quoi que ce soit, et environ 1050 tests
finissent Failed des deux côtés pour cette raison. La mesure de ce script ne dépend pas de ces
baselines : elle compare les deux passes **l'une contre l'autre**.

Les scripts de test téléchargent parfois des données distantes ; sans le réseau, ces tests
échouent de la même façon dans les deux passes.

`tools/clients/gmt_compare.py` compare alors :

1. le verdict de chaque entrée ctest : le même dans les deux passes, à l'identique ;
2. chaque fichier écrit par chaque test, récursivement (3974 fichiers) :
   - octet par octet pour tout ce qui n'est pas du PostScript ;
   - pour les ~1000 PostScript, le RMS du rendu 200 dpi de GraphicsMagick, avec la limite 0.003
     de `gmtest` ; si le rendu échoue (certains PostScript cassent Ghostscript 10.05 lui-même),
     les octets moins les commentaires DSC (dates) ;
3. tout ce qui ne passe pas est relancé **isolément**, une fois avec chaque bibliothèque, et la
   paire fraîche est comparée : GMT n'écrit pas toujours les mêmes fichiers dans une passe
   parallèle chargée (un psxy au bout d'un tube peut sauter sa couche, les solveurs itératifs de
   greenspline et potential réduisent dans un ordre qui suit l'ordonnanceur OpenMP). Si la paire
   fraîche est identique, la différence venait de la passe, pas de la bibliothèque.

Ne sont pas comparés, parce que GMT les rend différents de lui-même : les PDF (date de création
de Ghostscript), `gmt.history` et `*.tag` (horodatage), et tout ce qu'écrit
`grdtrack/crosstrack_geo` (un grdtrack parallèle dont l'ordre des lignes et la figure diffusent ;
deux exécutions de référence diffèrent).

## Résultat (GMT 6.5.0, GDAL 3.10.3 de Debian, GSHHG 2.3.7, DCW 2.2.0)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Mêmes verdicts pour les 1180 tests | 130 réussis, 1050 échoués (baselines absentes) | idem |
| Fichiers comparés | 3974 | 3974 |
| PostScript au-dessus de la limite RMS après relance isolée | 0 | 0 |
| Écarts d'octets après relance isolée | 0 | 0 |

Cinq fichiers diffèrent entre les deux passes complètes et sont le bruit de GMT sous charge, pas
un écart entre bibliothèques : `grdcut/origin/oblique` (une couche psxy sautée dans la passe de
référence), `greenspline/gspline_4`, `potential/growth_c`, `psxy/gaps` (solveurs itératifs,
ordonnanceur OpenMP). Chacun a été rejoué seul avec chaque bibliothèque : la paire fraîche est
identique. Une exécution isolée avec la bibliothèque de référence redonne d'ailleurs octet pour
octet le fichier de notre passe (oblique : 143137 octets des deux côtés, quand la passe chargée
de référence en avait écrit 139016).

Les tests qui atteignent libproj (comptés en interposant un compteur sur `proj_create`,
`proj_create_crs_to_crs` et `proj_trans` pendant toute une passe) : `mapproject/proj4`
(16 transformations), `grdimage` (7 jeux : l'UTM d'un `grdproject -J+proj=utm` relue depuis un
GeoTIFF, les textures reprojetées), `grdview`, `grdproject/units`, `grdmix`, `img/imgmap`,
`img/imgtrack`, `doc/scripts/GMT_mixing` — 49 processus, 213 `proj_create`, 17 `proj_trans`.

Aucune différence entre les deux bibliothèques n'a été trouvée, et rien n'a eu à être corrigé
dans proj-rs. Les erreurs rencontrées en montant ce banc d'essai (flex, bison, NetCDF,
GraphicsMagick, Ghostscript manquants sur la machine de test, baselines DVC inaccessibles,
`enable_testing` manquant) viennent de l'environnement, pas de proj-rs ; elles sont documentées
dans le script. Les seuls écarts de fichiers entre les deux passes sont ceux que GMT rend
différents entre deux exécutions de référence, et `gmt_compare.py` le vérifie par une double
relance isolée avant de conclure.
