# R (sf, terra) contre la bibliothèque C de proj-rs

R est le langage que la géographie statistique a choisi : `sf` (simple features, vecteurs) et
`terra` (raster et vecteurs) en sont les deux paquets géospatiaux de référence, tous deux
construits sur GDAL. Le code compilé de ces paquets ne lie pas libproj : c'est le GDAL du
système qui la lie, et la passe de ce script échange la bibliothèque sous lui par
`LD_LIBRARY_PATH` (même soname). GDAL fait tout le travail PROJ — WKT de `OGRSpatialReference`,
`OGRCreateCoordinateTransformation`, chaînes PROJ construites pour les étendues — et terra
appelle libproj elle-même à un endroit : `proj_pipelines`, qui liste les opérations de
coordination d'une paire de CRS par l'API C (`proj_create_operations`,
`proj_coordoperation_is_instantiable`), et le message de son option réseau
`projNetwork` qui signale les grilles de datum non téléchargeables.

Cette suite valide donc le côté GDAL de libproj (déjà couvert par GMT), mais aussi, pour la
première fois, le chemin du **listing des opérations** (`proj_create_operations` et la
vérification d'instantiabilité) appelé par un vrai client, réseaux activés puis grilles
manquantes sur le disque.

## Méthode

`tools/clients/r.sh` construit R 4.5.0 depuis sa source CRAN (version épinglée), installe
`sf` 1.1-3, `terra` 1.9-60 et `lwgeom` 0.2-17 depuis leurs sources CRAN (épinglées par leur sha256
sha256) avec leurs dépendances, puis lance chaque suite deux fois : avec la libproj de
PROJ 9.9.0 (la référence de `tools/build-reference.sh`) puis avec notre `libproj` au même
endroit. Les deux passes lisent le même `PROJ_DATA` (le `share/proj` de la référence, puis
les grilles de tests de proj-rs), réseau coupé (`PROJ_NETWORK=OFF`), puis terra l'active en
cours de route comme le fait son option `projNetwork`.

Les suites sont de deux genres :

- les 25 fichiers `*.R` des tests de sf (`R CMD BATCH --no-timing`), un fichier de sortie
  chacun ; ils comparent les constructions de CRS, les transformations, les allers-retours
  WKT et GeoPackage ;
- la suite tinytest de terra (59 fichiers de test, 981 vérifications) : projections de
  rasters et de vecteurs, `extract`, crop, appels `proj_pipelines` sur des paires de CRS
  nord-américaines dont les grilles de datum (NADCON, HTDP) manquent du disque.

La comparaison écrit chaque fichier de sortie normalisé (les durées de R et les lignes
`Cannot open https://cdn.proj.org/...: Network functionality not available` sorties),
comparés égaux : les valeurs de chaque test, les avertissements de R et les verdicts
sont comparés à l'identique, hors le compte de messages des deux côtés par ce qu'ils sont
un artefact des quantités de tentatives de chaque chemin d'API.

## Les erreurs avant les corrections

Chacune a été trouvée par un écart entre les deux passes, puis réduite en sonde C d'un seul
appel :

1. **L'ordre du listing.** `proj_pipelines("EPSG:4326", "EPSG:5070")` rendait, côté proj-rs,
   l'opération à grilles NADCON (1,5 m) *avant* le Helmert « NAD83 to WGS 84 (1) » (4 m, la
   null transformation) que PROJ met en premier. La cause : `gridpath::available` consultait
   la seule décision de téléchargement (`decide_download`, une ligne de `proj.db`), qui dit
   qu'une grille absente « est téléchargeable », au lieu d'ouvrir la grille comme le fait le
   tri de PROJ (`resolveFullFilename` → `NetworkFile::open`). Toutes les grilles CDN étaient
   donc « disponibles », les opérations à grilles gagnaient le tri. `project(vr, pipe=pp[2,])`
   de terra rendait NA partout avec proj-rs. Corrigé : `available` tente l'ouverture
   (`open_remote_grid`), le tri et le listing rendent maintenant les mêmes pipelines dans le
   même ordre, et les valeurs de tous les tests coincident.

2. **Les messages émis à l'intérieur d'un appel.** La tentative d'ouverture qui échoue
   (« `Cannot open https://cdn.proj.org/us_noaa_...tif: Network functionality not
   available` », 48 lignes dans la passe de listing) part du `pj_log` du contexte appelant :
   la bibliothèque Rust n'avait aucun canal pour ces messages, ils allaient nulle part.
   Ajout de `src/log_channel.rs` (un sink par fil, que la bibliothèque C installe à chacun
   de ses points d'entrée avec le logger du contexte) et de l'émission dans la tentative.

3. **L'instantiabilité ne tente pas.** À la création d'une paire
   (`proj_create_crs_to_crs`), proj-rs tentait 94 ouvertes (47 grilles deux fois : la
   disponibilité, puis `grid_serves` réessayait `available` puis `open_remote_grid`), là où
   PROJ n'en tente aucune : son marquage des opérations candidates croit `proj.db`
   (`lookForGridInfo(considerKnownGridsAsAvailable)`, `factory.cpp:3496-3510`) et n'ouvre qu'à
   la lecture, à la transformation. Corrigé par `gridpath::known_available` (sur disque, ou
   connue de `proj.db` réseau activé, sans tentative) aux six endroits qui marquent
   l'instantiabilité, suppression de la tentative doublée de `grid_serves`.

4. **L'échec de lecture silencieux.** À la transformation, une grille manquante rendait,
   côté proj-rs, une coordonnée NaN avec `errno` 0 (réussite) : GDAL poursuivait sans
   avertissement, quand PROJ rend l'infini avec `errno` 4099
   (`PROJ_ERR_OTHER_NETWORK_ERROR`) et un message `Cannot open`. Corrigé : la lecture d'une
   grille qui ne s'ouvre pas émet le message et échoue (`fetch_with_callbacks`,
   `read_grid`), et le message est le même que celui de PROJ (les deux grilles de la sonde
   C, une à une, dans le même ordre).

5. **Deux textes et un retour d'API.** `proj_list_get_count(NULL)` rendait -1 (PROJ : 0), et
   les arguments manquants de `proj_create_operations` et de sept constructeurs disaient
   `missing argument`/`wrong or missing argument` (PROJ : `missing required input`). Un
   lock mutex pris deux fois dans ce correctif (la garde du scrutinee d'un `match` vit
   jusqu'à la fin du bloc) bloquait la première transformation d'une grille ; le fix l'a
   délié avant le second accès.


## Résultat (R 4.5.0, sf 1.1-3, terra 1.9-60, GDAL 3.10.3 de Debian)

| | PROJ 9.9.0 | proj-rs |
|---|---:|---:|
| Fichiers de sortie sf (25) identiques, normalisés | 25 | 25 |
| tinytest terra : « All ok » | 981 résultats | 981 résultats |
| Messages du listing `proj_pipelines` (réseau activé) | 48 | 48 |
| Tentatives à la création d'une paire | 0 | 0 |
| `sf-...` et échecs de `R CMD BATCH` | aucun | aucun |

## Ce qui reste

Un seul écart, sans effet sur les valeurs : dans `project(a, b, pipe=pp[1,])` de terra (le
pipeline `aea` « Inverse of NAD83 to WGS 84 (1) + Conus Albers », EPSG:4326 → EPSG:5070),
les quatre coins de l'étendue estimée échouent des deux côtés (latitudes 20 et 60 hors du
domaine), mais PROJ garde collant sur le contexte l'errno 4099 de la phase de listing et
GDAL avertit deux fois (`Reprojection failed, err = 4099`, `transformation produced no valid
coordinates`) avant de retomber sur les mêmes valeurs ; proj-rs rend la même étendue finale
sans les deux avertissements. Les rasters finaux sont identiques byte pour byte (sommes,
NA, digests), et une sonde C d'un appel isolé rend le même comportement point par point des
deux côtés.
